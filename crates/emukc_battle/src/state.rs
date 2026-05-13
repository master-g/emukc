use crate::outcome::{calculate_mvp, calculate_win_rank, verify_protected_ships_alive};
use crate::targeting::any_alive;
use crate::types::{
    BattleContext, BattleHougeki, BattleKouku, BattleOpeningAttack, BattleOutcome, BattlePacket,
    BattleRaigeki, BattleRuntimeShip, BattleSimulation, BattleType, NightBattlePacket,
    NightBattleSimulation,
};

/// All mutable state for a single battle simulation.
///
/// Created from a [`BattleContext`], mutated by phase functions, then
/// consumed by [`finalize_day`](Self::finalize_day) or
/// [`finalize_night`](Self::finalize_night) to produce the simulation output.
///
/// `friendly` and `enemy` are `pub(crate)` because every phase function needs
/// `&mut` access to them. All other fields are private with setters.
pub(crate) struct BattleState {
    pub(crate) friendly: Vec<BattleRuntimeShip>,
    pub(crate) enemy: Vec<BattleRuntimeShip>,

    battle_type: BattleType,
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: super::types::EngagementType,

    kouku: Option<BattleKouku>,
    opening_attack: Option<BattleOpeningAttack>,
    opening_taisen: Option<BattleHougeki>,
    hougeki1: Option<BattleHougeki>,
    hougeki2: Option<BattleHougeki>,
    raigeki: Option<BattleRaigeki>,

    stage_flag: [i64; 3],
    hourai_flag: [i64; 4],
    opening_taisen_flag: i64,
    has_bb_class_at_start: bool,
}

impl BattleState {
    /// Build initial state from a battle context.
    pub fn from_context(context: BattleContext) -> Self {
        let is_sortie = context.is_sortie;
        let friendly = context
            .friend_ships
            .into_iter()
            .map(|s| BattleRuntimeShip::new(s, true, is_sortie))
            .collect::<Vec<_>>();
        let enemy = context
            .enemy_ships
            .into_iter()
            .map(|s| BattleRuntimeShip::new(s, false, is_sortie))
            .collect::<Vec<_>>();

        Self {
            friendly,
            enemy,
            battle_type: context.battle_type,
            friendly_formation_id: context.friendly_formation_id,
            enemy_formation_id: context.enemy_formation_id,
            engagement: context.engagement,
            kouku: None,
            opening_attack: None,
            opening_taisen: None,
            hougeki1: None,
            hougeki2: None,
            raigeki: None,
            stage_flag: [0, 0, 0],
            hourai_flag: [0, 0, 0, 0],
            opening_taisen_flag: 0,
            has_bb_class_at_start: false,
        }
    }

    /// Build minimal state for night battle finalization.
    /// Avoids the full `BattleContext` → runtime-ship pipeline when the ships
    /// have already been mutated by day battle phases.
    pub fn for_night(
        friendly: Vec<BattleRuntimeShip>,
        enemy: Vec<BattleRuntimeShip>,
        friendly_formation_id: i64,
        enemy_formation_id: i64,
        engagement: super::types::EngagementType,
    ) -> Self {
        Self {
            friendly,
            enemy,
            battle_type: BattleType::Normal,
            friendly_formation_id,
            enemy_formation_id,
            engagement,
            kouku: None,
            opening_attack: None,
            opening_taisen: None,
            hougeki1: None,
            hougeki2: None,
            raigeki: None,
            stage_flag: [0, 0, 0],
            hourai_flag: [0, 0, 0, 0],
            opening_taisen_flag: 0,
            has_bb_class_at_start: false,
        }
    }

    // -- Read accessors (for phase dispatch) --

    pub(crate) fn battle_type(&self) -> BattleType {
        self.battle_type
    }

    pub(crate) fn friendly_formation_id(&self) -> i64 {
        self.friendly_formation_id
    }

    pub(crate) fn enemy_formation_id(&self) -> i64 {
        self.enemy_formation_id
    }

    pub(crate) fn engagement(&self) -> super::types::EngagementType {
        self.engagement
    }

    // -- Setters (for phase functions to write outputs) --

    pub(crate) fn set_kouku(&mut self, kouku: BattleKouku) {
        self.kouku = Some(kouku);
    }

    pub(crate) fn set_opening_attack(&mut self, attack: Option<BattleOpeningAttack>) {
        self.opening_attack = attack;
    }

    pub(crate) fn set_opening_taisen(&mut self, taisen: Option<BattleHougeki>) {
        self.opening_taisen = taisen;
    }

    pub(crate) fn set_opening_taisen_flag(&mut self, flag: bool) {
        self.opening_taisen_flag = i64::from(flag);
    }

    pub(crate) fn set_hougeki1(&mut self, hougeki: Option<BattleHougeki>) {
        self.hougeki1 = hougeki;
    }

    pub(crate) fn set_hougeki2(&mut self, hougeki: Option<BattleHougeki>) {
        self.hougeki2 = hougeki;
    }

    pub(crate) fn set_raigeki(&mut self, raigeki: Option<BattleRaigeki>) {
        self.raigeki = raigeki;
    }

    pub(crate) fn set_stage_flag(&mut self, flags: [i64; 3]) {
        self.stage_flag = flags;
    }

    pub(crate) fn set_hourai_flag(&mut self, index: usize, value: i64) {
        debug_assert!(index < 4, "hourai_flag index out of bounds: {index}");
        self.hourai_flag[index] = value;
    }

    pub(crate) fn set_has_bb_class_at_start(&mut self, value: bool) {
        self.has_bb_class_at_start = value;
    }

    pub(crate) fn has_bb_class_at_start(&self) -> bool {
        self.has_bb_class_at_start
    }

    // -- Finalizers --

    /// Consume state, verify invariants, produce the day battle simulation result.
    pub fn finalize_day(self) -> BattleSimulation {
        verify_protected_ships_alive(&self.friendly);

        let can_midnight = matches!(self.battle_type, BattleType::Normal | BattleType::AirBattle)
            && any_alive(&self.friendly)
            && any_alive(&self.enemy);

        let packet = BattlePacket {
            formation: [
                self.friendly_formation_id,
                self.enemy_formation_id,
                self.engagement.api_id(),
            ],
            friendly_nowhps: self.friendly.iter().map(|ship| ship.hp().max(0)).collect(),
            enemy_nowhps: self.enemy.iter().map(|ship| ship.hp().max(0)).collect(),
            smoke_type: 0,
            balloon_cell: 0,
            atoll_cell: 0,
            midnight_flag: i64::from(can_midnight),
            search: [1, 1],
            stage_flag: self.stage_flag,
            kouku: self.kouku,
            opening_taisen_flag: self.opening_taisen_flag,
            opening_taisen: self.opening_taisen,
            opening_flag: i64::from(self.opening_attack.is_some()),
            opening_attack: self.opening_attack,
            hourai_flag: self.hourai_flag,
            hougeki1: self.hougeki1,
            hougeki2: self.hougeki2,
            hougeki3: None,
            raigeki: self.raigeki,
        };

        let outcome = BattleOutcome {
            win_rank: calculate_win_rank(&self.friendly, &self.enemy),
            mvp: calculate_mvp(&self.friendly),
            can_midnight,
        };

        BattleSimulation {
            friendly: self.friendly,
            enemy: self.enemy,
            packet,
            outcome,
        }
    }

    /// Consume state, verify invariants, produce the night battle simulation result.
    pub fn finalize_night(
        self,
        friendly_nowhps: Vec<i64>,
        friendly_maxhps: Vec<i64>,
        enemy_nowhps: Vec<i64>,
        enemy_maxhps: Vec<i64>,
        hougeki: Option<crate::types::BattleNightHougeki>,
    ) -> NightBattleSimulation {
        verify_protected_ships_alive(&self.friendly);

        let outcome = BattleOutcome {
            win_rank: calculate_win_rank(&self.friendly, &self.enemy),
            mvp: calculate_mvp(&self.friendly),
            can_midnight: false,
        };

        let packet = NightBattlePacket {
            formation: [
                self.friendly_formation_id,
                self.enemy_formation_id,
                self.engagement.api_id(),
            ],
            friendly_nowhps,
            friendly_maxhps,
            enemy_nowhps,
            enemy_maxhps,
            touch_plane: [-1, -1],
            flare_pos: [-1, -1],
            hougeki,
        };

        NightBattleSimulation {
            friendly: self.friendly,
            enemy: self.enemy,
            packet,
            outcome,
        }
    }
}
