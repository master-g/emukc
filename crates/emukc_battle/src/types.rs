use serde::Serialize;

use crate::random::BattleRng;
use emukc_model::kc2::{KcApiShip, KcApiSlotItem};

/// Parameters for a shelling side simulation.
pub(crate) struct ShellingParams {
    pub attacker_is_enemy: bool,
    pub formation_id: i64,
    pub engagement: EngagementType,
    pub phase: BattlePhase,
}

/// Mutable output buffers for an airstrike phase.
pub(crate) struct AirstrikeOutput<'a> {
    pub damage: &'a mut [i64],
    pub bak_targets: &'a mut [i64],
    pub rai_targets: &'a mut [i64],
}

/// Parameters for night battle shelling simulation.
pub(crate) struct NightBattleParams<'a> {
    pub friendly_formation_id: i64,
    pub enemy_formation_id: i64,
    pub engagement: EngagementType,
    pub air_state: Option<&'a AirState>,
}

/// Controls which phases execute in a day battle simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleType {
    /// Normal day battle: kouku -> OASW -> opening torpedo -> shelling x 2 -> closing torpedo.
    Normal,
    /// Air battle only (航空戦): kouku + OASW, no shelling / torpedo.
    AirBattle,
    /// Long-distance air raid (長距離空襲): kouku only, no OASW / shelling / torpedo.
    LdAirBattle,
    /// Long-distance shelling (長距離砲撃): shelling only, no kouku / torpedo.
    LdShooting,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngagementType {
    SameCourse,
    HeadOn,
    TAdvantage,
    TDisadvantage,
}

impl EngagementType {
    pub const fn api_id(self) -> i64 {
        match self {
            Self::SameCourse => 1,
            Self::HeadOn => 2,
            Self::TAdvantage => 3,
            Self::TDisadvantage => 4,
        }
    }

    pub const fn modifier(self) -> f64 {
        match self {
            Self::SameCourse => 1.0,
            Self::HeadOn => 0.8,
            Self::TAdvantage => 1.2,
            Self::TDisadvantage => 0.6,
        }
    }

    pub const fn from_api_id(api_id: i64) -> Option<Self> {
        match api_id {
            1 => Some(Self::SameCourse),
            2 => Some(Self::HeadOn),
            3 => Some(Self::TAdvantage),
            4 => Some(Self::TDisadvantage),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BattleShipInput {
    pub ship: KcApiShip,
    pub slot_items: Vec<KcApiSlotItem>,
    pub effect_list: Vec<i64>,
    pub married: bool,
}

#[derive(Debug, Clone)]
pub struct BattleRuntimeShip {
    pub ship: KcApiShip,
    pub slot_items: Vec<KcApiSlotItem>,
    pub effect_list: Vec<i64>,
    /// Current HP -- only mutable through [`apply_damage`](Self::apply_damage).
    pub(crate) current_hp: i64,
    /// HP at the start of this battle node (before any combat phases).
    /// Used to determine sinking protection eligibility.
    pub entry_hp: i64,
    pub damage_dealt: i64,
    /// Whether this ship belongs to the player (friendly) side.
    pub(crate) is_friendly: bool,
    /// Whether this battle is a sortie (true) or practice (false).
    /// Sinking protection only applies during sorties.
    pub(crate) is_sortie: bool,
    pub married: bool,
}

impl BattleRuntimeShip {
    /// Create a runtime ship for battle simulation.
    pub fn new(input: BattleShipInput, is_friendly: bool, is_sortie: bool) -> Self {
        Self {
            current_hp: input.ship.api_nowhp,
            entry_hp: input.ship.api_nowhp,
            damage_dealt: 0,
            ship: input.ship,
            slot_items: input.slot_items,
            effect_list: input.effect_list,
            is_friendly,
            is_sortie,
            married: input.married,
        }
    }

    /// Current HP (read-only).
    pub fn hp(&self) -> i64 {
        self.current_hp
    }

    pub fn is_alive(&self) -> bool {
        self.current_hp > 0
    }

    pub fn is_sunk(&self) -> bool {
        self.current_hp <= 0
    }

    /// Apply damage with sinking protection (轟沈ストッパー).
    ///
    /// In real `KanColle`:
    /// - Friendly ships that were **not** in taiha (HP <= 25% max) at the start of
    ///   the battle node cannot be sunk. Lethal damage is replaced with
    ///   proportional damage: `floor(0.5 * H + 0.3 * rand(0..H))`.
    /// - The flagship (index 0) can **never** be sunk regardless of HP state.
    /// - Protection only applies to friendly ships during sorties (not practice).
    ///
    /// Returns `(raw_damage, effective_damage)` where raw is the input damage
    /// and effective is the HP actually subtracted (after clamping/protection).
    pub(crate) fn apply_damage(
        &mut self,
        rng: &mut impl BattleRng,
        raw_damage: i64,
        ship_index: usize,
    ) -> (i64, i64) {
        if self.is_sunk() {
            return (0, 0);
        }

        let effective = raw_damage.min(self.current_hp);

        // Sinking protection only applies to friendly ships during sorties.
        if self.is_friendly && self.is_sortie && effective >= self.current_hp {
            let is_flagship = ship_index == 0;
            // Taiha threshold: HP <= 25% of max at node entry.
            let was_taiha_at_entry = self.entry_hp * 4 <= self.ship.api_maxhp;
            let is_protected = is_flagship || !was_taiha_at_entry;

            if is_protected {
                // Replace lethal damage with proportional damage (割合ダメージ).
                // Formula uses entry_hp as base: (H / 2) + (rand_part * 3) / 10
                // Clamped to [0, current_hp - 1] to guarantee survival.
                let h = self.entry_hp;
                let rand_part = if h > 1 {
                    rng.roll_range(0, h)
                } else {
                    0
                };
                let proportional = (h / 2) + (rand_part * 3) / 10;
                let dealt = proportional.min(self.current_hp - 1).max(0);
                self.current_hp -= dealt;
                return (raw_damage, dealt);
            }
        }

        self.current_hp -= effective;
        (raw_damage, effective)
    }
}

#[cfg(test)]
impl From<BattleShipInput> for BattleRuntimeShip {
    /// Convenience conversion for tests.
    /// Defaults to friendly sortie ship (sinking protection enabled).
    /// Use `BattleRuntimeShip::new(input, is_enemy, is_sortie)` for specific contexts.
    fn from(input: BattleShipInput) -> Self {
        Self::new(input, false, true)
    }
}

#[derive(Debug, Clone)]
pub struct BattleContext {
    pub battle_type: BattleType,
    /// Whether this is a sortie battle (true) or practice (false).
    /// Sinking protection only applies during sorties.
    pub is_sortie: bool,
    pub friendly_formation_id: i64,
    pub enemy_formation_id: i64,
    pub engagement: EngagementType,
    pub friend_ships: Vec<BattleShipInput>,
    pub enemy_ships: Vec<BattleShipInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePhase {
    OpeningTorpedo,
    DayShelling,
    ClosingTorpedo,
    NightShelling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TargetClass {
    SurfaceShip,
    Installation,
    PtBoat,
    Submarine,
}

impl TargetClass {
    pub(crate) const fn is_submarine(self) -> bool {
        matches!(self, Self::Submarine)
    }

    pub(crate) const fn is_surface_like(self) -> bool {
        !self.is_submarine()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirState {
    Supremacy,
    Superiority,
    Parity,
    Denial,
    Incapability,
}

impl AirState {
    pub(crate) fn from_power(friendly: i64, enemy: i64) -> Self {
        if enemy == 0 && friendly == 0 {
            return Self::Parity;
        }
        if enemy == 0 {
            return Self::Supremacy;
        }
        // Thresholds ordered from most favorable to least:
        // Supremacy:    friendly >= 3 x enemy
        // Superiority:  friendly >= 1.5 x enemy  (2*friendly >= 3*enemy)
        // ... Parity in the middle ...
        // Denial:       enemy >= 1.5 x friendly  (3*friendly <= 2*enemy)
        // Incapability: enemy >= 3 x friendly    (3*friendly <= enemy)
        if friendly >= 3 * enemy {
            Self::Supremacy
        } else if 2 * friendly >= 3 * enemy {
            Self::Superiority
        } else if 3 * friendly <= enemy {
            Self::Incapability
        } else if 3 * friendly <= 2 * enemy {
            Self::Denial
        } else {
            Self::Parity
        }
    }

    pub(crate) fn api_disp_seiku(self) -> i64 {
        match self {
            Self::Supremacy => 1,
            Self::Superiority => 2,
            Self::Parity => 0,
            Self::Denial => 3,
            Self::Incapability => 4,
        }
    }

    pub fn from_api_disp_seiku(value: i64) -> Option<Self> {
        match value {
            1 => Some(Self::Supremacy),
            2 => Some(Self::Superiority),
            0 => Some(Self::Parity),
            3 => Some(Self::Denial),
            4 => Some(Self::Incapability),
            _ => None,
        }
    }

    pub(crate) fn stage1_friendly_loss_ratio(self) -> (f64, f64) {
        match self {
            Self::Supremacy => (0.0, 0.04),
            Self::Superiority => (0.02, 0.08),
            Self::Parity => (0.04, 0.12),
            Self::Denial => (0.08, 0.18),
            Self::Incapability => (0.20, 0.36),
        }
    }

    pub(crate) fn stage1_enemy_loss_ratio(self) -> (f64, f64) {
        match self {
            Self::Supremacy => (0.20, 0.36),
            Self::Superiority => (0.08, 0.18),
            Self::Parity => (0.04, 0.12),
            Self::Denial => (0.02, 0.08),
            Self::Incapability => (0.0, 0.04),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttackCapability {
    CannotAttack,
    SurfaceOnly,
    BothPreferSubmarine,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKouku {
    pub api_plane_from: [Vec<i64>; 2],
    pub api_stage1: BattleKoukuStage1,
    pub api_stage2: BattleKoukuStage2,
    pub api_stage3: BattleKoukuStage3,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage1 {
    pub api_f_count: i64,
    pub api_f_lostcount: i64,
    pub api_e_count: i64,
    pub api_e_lostcount: i64,
    pub api_disp_seiku: i64,
    pub api_touch_plane: [i64; 2],
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage2 {
    pub api_f_count: i64,
    pub api_f_lostcount: i64,
    pub api_e_count: i64,
    pub api_e_lostcount: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleKoukuStage3 {
    pub api_frai: Vec<i64>,
    pub api_erai: Vec<i64>,
    pub api_fbak: Vec<i64>,
    pub api_ebak: Vec<i64>,
    pub api_fcl_flag: Vec<i64>,
    pub api_ecl_flag: Vec<i64>,
    pub api_fdam: Vec<i64>,
    pub api_edam: Vec<i64>,
    pub api_f_sp_list: Vec<Option<i64>>,
    pub api_e_sp_list: Vec<Option<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleOpeningAttack {
    pub api_frai_list_items: Vec<Option<Vec<i64>>>,
    pub api_fcl_list_items: Vec<Option<Vec<i64>>>,
    pub api_fdam: Vec<i64>,
    pub api_fydam_list_items: Vec<Option<Vec<i64>>>,
    pub api_erai_list_items: Vec<Option<Vec<i64>>>,
    pub api_ecl_list_items: Vec<Option<Vec<i64>>>,
    pub api_edam: Vec<i64>,
    pub api_eydam_list_items: Vec<Option<Vec<i64>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleHougeki {
    pub api_at_eflag: Vec<i64>,
    pub api_at_list: Vec<i64>,
    pub api_at_type: Vec<i64>,
    pub api_df_list: Vec<Vec<i64>>,
    pub api_si_list: Vec<Vec<i64>>,
    pub api_cl_list: Vec<Vec<i64>>,
    pub api_damage: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleNightHougeki {
    pub api_at_eflag: Vec<i64>,
    pub api_at_list: Vec<i64>,
    pub api_n_mother_list: Vec<i64>,
    pub api_df_list: Vec<Vec<i64>>,
    pub api_si_list: Vec<Vec<i64>>,
    pub api_cl_list: Vec<Vec<i64>>,
    pub api_sp_list: Vec<i64>,
    pub api_damage: Vec<Vec<i64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BattleRaigeki {
    pub api_frai: Vec<i64>,
    pub api_fcl: Vec<i64>,
    pub api_fdam: Vec<i64>,
    pub api_fydam: Vec<i64>,
    pub api_erai: Vec<i64>,
    pub api_ecl: Vec<i64>,
    pub api_edam: Vec<i64>,
    pub api_eydam: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TorpedoAttackerSide {
    Friendly,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TorpedoHit {
    pub(crate) attacker_index: usize,
    pub(crate) defender_index: usize,
    pub(crate) damage: i64,
}

impl BattleOpeningAttack {
    pub(crate) fn blank(len: usize) -> Self {
        Self {
            api_frai_list_items: vec![None; len],
            api_fcl_list_items: vec![None; len],
            api_fdam: vec![0; len],
            api_fydam_list_items: vec![None; len],
            api_erai_list_items: vec![None; len],
            api_ecl_list_items: vec![None; len],
            api_edam: vec![0; len],
            api_eydam_list_items: vec![None; len],
        }
    }

    pub(crate) fn record_torpedo_hit(
        &mut self,
        attacker_side: TorpedoAttackerSide,
        hit: TorpedoHit,
    ) {
        match attacker_side {
            TorpedoAttackerSide::Friendly => {
                self.api_frai_list_items[hit.attacker_index] =
                    Some(vec![hit.defender_index as i64]);
                self.api_fcl_list_items[hit.attacker_index] = Some(vec![1]);
                self.api_fydam_list_items[hit.attacker_index] = Some(vec![hit.damage]);
                self.api_edam[hit.defender_index] += hit.damage;
            }
            TorpedoAttackerSide::Enemy => {
                self.api_erai_list_items[hit.attacker_index] =
                    Some(vec![hit.defender_index as i64]);
                self.api_ecl_list_items[hit.attacker_index] = Some(vec![1]);
                self.api_eydam_list_items[hit.attacker_index] = Some(vec![hit.damage]);
                self.api_fdam[hit.defender_index] += hit.damage;
            }
        }
    }
}

impl BattleRaigeki {
    pub(crate) fn blank(len: usize) -> Self {
        Self {
            api_frai: vec![-1; len],
            api_fcl: vec![0; len],
            api_fdam: vec![0; len],
            api_fydam: vec![0; len],
            api_erai: vec![-1; len],
            api_ecl: vec![0; len],
            api_edam: vec![0; len],
            api_eydam: vec![0; len],
        }
    }

    pub(crate) fn record_torpedo_hit(
        &mut self,
        attacker_side: TorpedoAttackerSide,
        hit: TorpedoHit,
    ) {
        match attacker_side {
            TorpedoAttackerSide::Friendly => {
                self.api_frai[hit.attacker_index] = hit.defender_index as i64;
                self.api_fcl[hit.attacker_index] = 1;
                self.api_fydam[hit.attacker_index] = hit.damage;
                self.api_edam[hit.defender_index] += hit.damage;
            }
            TorpedoAttackerSide::Enemy => {
                self.api_erai[hit.attacker_index] = hit.defender_index as i64;
                self.api_ecl[hit.attacker_index] = 1;
                self.api_eydam[hit.attacker_index] = hit.damage;
                self.api_fdam[hit.defender_index] += hit.damage;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct BattlePacket {
    pub formation: [i64; 3],
    pub friendly_nowhps: Vec<i64>,
    pub enemy_nowhps: Vec<i64>,
    pub smoke_type: i64,
    pub balloon_cell: i64,
    pub atoll_cell: i64,
    pub midnight_flag: i64,
    pub search: [i64; 2],
    pub stage_flag: [i64; 3],
    pub kouku: Option<BattleKouku>,
    pub opening_taisen_flag: i64,
    pub opening_taisen: Option<BattleHougeki>,
    pub opening_flag: i64,
    pub opening_attack: Option<BattleOpeningAttack>,
    pub hourai_flag: [i64; 4],
    pub hougeki1: Option<BattleHougeki>,
    pub hougeki2: Option<BattleHougeki>,
    pub hougeki3: Option<BattleHougeki>,
    pub raigeki: Option<BattleRaigeki>,
}

#[derive(Debug, Clone)]
pub struct BattleOutcome {
    pub win_rank: String,
    pub mvp: i64,
    #[allow(dead_code)]
    pub can_midnight: bool,
}

#[derive(Debug, Clone)]
pub struct BattleSimulation {
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub packet: BattlePacket,
    pub outcome: BattleOutcome,
}

#[derive(Debug, Clone)]
pub struct NightBattlePacket {
    pub formation: [i64; 3],
    pub friendly_nowhps: Vec<i64>,
    pub friendly_maxhps: Vec<i64>,
    pub enemy_nowhps: Vec<i64>,
    pub enemy_maxhps: Vec<i64>,
    pub touch_plane: [i64; 2],
    pub flare_pos: [i64; 2],
    pub hougeki: Option<BattleNightHougeki>,
}

#[derive(Debug, Clone)]
pub struct NightBattleSimulation {
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub packet: NightBattlePacket,
    pub outcome: BattleOutcome,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    // ── AirState tests ──────────────────────────────────────────────

    #[test]
    fn air_state_supremacy_when_friendly_triples_enemy() {
        assert_eq!(AirState::from_power(300, 100), AirState::Supremacy);
        assert_eq!(AirState::from_power(300, 0), AirState::Supremacy);
        assert_eq!(AirState::from_power(301, 100), AirState::Supremacy);
    }

    #[test]
    fn air_state_superiority_when_friendly_exceeds_1_5x() {
        assert_eq!(AirState::from_power(150, 100), AirState::Superiority);
        assert_eq!(AirState::from_power(200, 100), AirState::Superiority);
        assert_eq!(AirState::from_power(299, 100), AirState::Superiority);
    }

    #[test]
    fn air_state_parity_in_middle_range() {
        assert_eq!(AirState::from_power(0, 0), AirState::Parity);
        assert_eq!(AirState::from_power(100, 100), AirState::Parity);
        assert_eq!(AirState::from_power(149, 100), AirState::Parity);
        assert_eq!(AirState::from_power(100, 149), AirState::Parity);
    }

    #[test]
    fn air_state_denial_when_enemy_exceeds_1_5x() {
        assert_eq!(AirState::from_power(100, 150), AirState::Denial);
        assert_eq!(AirState::from_power(100, 200), AirState::Denial);
    }

    #[test]
    fn air_state_incapability_when_enemy_triples_friendly() {
        assert_eq!(AirState::from_power(100, 300), AirState::Incapability);
        assert_eq!(AirState::from_power(100, 301), AirState::Incapability);
        assert_eq!(AirState::from_power(0, 100), AirState::Incapability);
    }

    #[test]
    fn air_state_api_disp_seiku_values() {
        assert_eq!(AirState::Supremacy.api_disp_seiku(), 1);
        assert_eq!(AirState::Superiority.api_disp_seiku(), 2);
        assert_eq!(AirState::Parity.api_disp_seiku(), 0);
        assert_eq!(AirState::Denial.api_disp_seiku(), 3);
        assert_eq!(AirState::Incapability.api_disp_seiku(), 4);
    }

    // ── Sinking protection tests ────────────────────────────────────

    #[test]
    fn sinking_protection_saves_non_taiha_ship_in_sortie() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship(30, 30, 30, 40);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 1);
        assert!(ship.hp() >= 1, "ship must survive with sinking protection");
        assert!(effective < 30, "effective damage must be less than current HP");
        assert_eq!(raw, 999, "raw should show full input damage");
    }

    #[test]
    fn flagship_always_survives_even_when_taiha() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship(5, 5, 5, 40);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 0);
        assert!(ship.hp() >= 1, "flagship must always survive");
        assert!(effective < 5);
        assert_eq!(raw, 999);
    }

    #[test]
    fn taiha_advance_ship_can_be_sunk() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship(5, 5, 5, 40);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 1);
        assert_eq!(ship.hp(), 0, "taiha-advance ship should be sunk");
        assert_eq!(effective, 5);
        assert_eq!(raw, 999);
    }

    #[test]
    fn practice_never_triggers_sinking_protection() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship_ctx(30, 30, 30, 40, true, false);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 1);
        assert_eq!(ship.hp(), 0, "practice uses normal damage clamping");
        assert_eq!(effective, 30);
        assert_eq!(raw, 999);
    }

    #[test]
    fn enemy_ships_never_get_sinking_protection() {
        let mut rng = crate::random::SeededRng::new(42);
        let mut ship = make_test_ship_ctx(30, 30, 30, 40, false, true);
        let (raw, effective) = ship.apply_damage(&mut rng, 999, 0);
        assert_eq!(ship.hp(), 0, "enemy ships should be sinkable");
        assert_eq!(effective, 30);
        assert_eq!(raw, 999);
    }

    #[test]
    fn flagship_is_always_protected_from_sinking() {
        let mut ship = make_test_ship_ctx(10, 10, 10, 30, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert!(effective > 0, "flagship should take proportional damage");
        assert!(ship.current_hp > 0, "flagship must survive");
        assert!(ship.current_hp < ship.entry_hp, "should be proportional, not full damage");
        assert_eq!(raw, 100, "raw should show full input");
    }

    #[test]
    fn flagship_at_1hp_survives_lethal_damage() {
        let mut ship = make_test_ship_ctx(1, 5, 1, 30, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert_eq!(effective, 0, "at 1 HP, protection reduces damage to 0");
        assert_eq!(ship.current_hp, 1, "flagship must survive");
        assert_eq!(raw, 100);
    }

    #[test]
    fn non_taiha_ship_is_protected_from_sinking() {
        let mut ship = make_test_ship_ctx(10, 20, 10, 30, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 2);
        assert!(effective > 0, "non-taiha ship should take proportional damage");
        assert!(ship.current_hp > 0, "non-taiha ship must survive");
        assert_eq!(raw, 100);
    }

    #[test]
    fn taiha_non_flagship_can_be_sunk() {
        let entry_hp = 5;
        let max_hp = 30;
        let mut ship = make_test_ship_ctx(entry_hp, entry_hp, entry_hp, max_hp, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 2);
        assert_eq!(ship.current_hp, 0, "taiha non-flagship should be sunk");
        assert_eq!(effective, 5);
        assert_eq!(raw, 100);
    }

    #[test]
    fn protection_uses_entry_hp_not_current_hp() {
        let max_hp = 40;
        let mut ship = make_test_ship_ctx(10, 30, 10, max_hp, true, true);
        let mut rng = crate::random::SeededRng::new(123);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 1);
        assert!(effective > 0);
        assert!(ship.current_hp > 0, "should survive due to protection");
        assert!(
            ship.current_hp <= 30,
            "remaining HP should be based on entry_hp (30), not current_hp (10)"
        );
        assert_eq!(raw, 100);
    }

    #[test]
    fn enemy_ships_get_no_protection() {
        let mut ship = make_test_ship_ctx(1, 1, 1, 30, false, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert_eq!(effective, 1, "enemy ship should take full effective damage");
        assert_eq!(raw, 100, "raw should show overkill");
        assert_eq!(ship.current_hp, 0, "enemy ship should be sunk");
    }

    #[test]
    fn practice_ships_get_no_protection() {
        let mut ship = make_test_ship_ctx(1, 1, 1, 30, true, false);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert_eq!(effective, 1, "practice ship should take full effective damage");
        assert_eq!(raw, 100, "raw should show overkill");
        assert_eq!(ship.current_hp, 0, "practice ship should be sunk");
    }

    #[test]
    fn overkill_shows_raw_damage() {
        let mut ship = make_test_ship_ctx(5, 5, 5, 30, false, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 100, 0);
        assert_eq!(raw, 100, "raw should show full input damage");
        assert_eq!(effective, 5, "effective capped to current HP");
        assert_eq!(ship.current_hp, 0, "ship should be sunk");
    }

    #[test]
    fn protection_shows_raw_but_reduces_hp_proportionally() {
        let mut ship = make_test_ship_ctx(10, 10, 10, 30, true, true);
        let mut rng = crate::random::SeededRng::new(42);
        let (raw, effective) = ship.apply_damage(&mut rng, 200, 0);
        assert_eq!(raw, 200, "raw should show full lethal input");
        assert!(effective < 10, "effective should be proportional, not lethal");
        assert!(ship.current_hp > 0, "flagship must survive");
    }

    // ── Payload builder tests ───────────────────────────────────────

    #[test]
    fn opening_torpedo_payload_builder_routes_damage_by_attacker_side() {
        let mut payload = BattleOpeningAttack::blank(2);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Friendly,
            TorpedoHit {
                attacker_index: 1,
                defender_index: 0,
                damage: 21,
            },
        );
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Enemy,
            TorpedoHit {
                attacker_index: 0,
                defender_index: 1,
                damage: 34,
            },
        );
        let opening = payload;

        assert_eq!(opening.api_frai_list_items[1], Some(vec![0]));
        assert_eq!(opening.api_fydam_list_items[1], Some(vec![21]));
        assert_eq!(opening.api_eydam_list_items[1], None);
        assert_eq!(opening.api_edam[0], 21);
        assert_eq!(opening.api_erai_list_items[0], Some(vec![1]));
        assert_eq!(opening.api_eydam_list_items[0], Some(vec![34]));
        assert_eq!(opening.api_fydam_list_items[0], None);
        assert_eq!(opening.api_fdam[1], 34);
    }

    #[test]
    fn raigeki_payload_builder_routes_damage_by_attacker_side() {
        let mut payload = BattleRaigeki::blank(2);
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Friendly,
            TorpedoHit {
                attacker_index: 1,
                defender_index: 0,
                damage: 21,
            },
        );
        payload.record_torpedo_hit(
            TorpedoAttackerSide::Enemy,
            TorpedoHit {
                attacker_index: 0,
                defender_index: 1,
                damage: 34,
            },
        );
        let raigeki = payload;

        assert_eq!(raigeki.api_frai[1], 0);
        assert_eq!(raigeki.api_fydam[1], 21);
        assert_eq!(raigeki.api_eydam[1], 0);
        assert_eq!(raigeki.api_edam[0], 21);
        assert_eq!(raigeki.api_erai[0], 1);
        assert_eq!(raigeki.api_eydam[0], 34);
        assert_eq!(raigeki.api_fydam[0], 0);
        assert_eq!(raigeki.api_fdam[1], 34);
    }
}
