//! Runtime battle types — ship state, battle context, and simulation output.
//! These types carry mutable battle state and top-level simulation results.

use emukc_model::kc2::{KcApiShip, KcApiSlotItem, KcSortieResultRank};

use super::domain::{AirState, BattleType, EngagementType};
use super::packet::{
    BattleHougeki, BattleKouku, BattleNightHougeki, BattleOpeningAttack, BattleRaigeki,
};
use crate::random::BattleRng;

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

/// Input parameters for a day battle simulation.
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

/// Input parameters for [`simulate_night`](crate::simulation::simulate_night).
pub struct NightBattleInput {
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub friendly_formation_id: i64,
    pub enemy_formation_id: i64,
    pub engagement: EngagementType,
    pub air_state: Option<AirState>,
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

/// Battle result: win rank, MVP ship index, and midnight eligibility.
#[derive(Debug, Clone)]
pub struct BattleOutcome {
    pub win_rank: KcSortieResultRank,
    pub mvp: i64,
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
