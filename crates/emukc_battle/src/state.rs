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
/// consumed by [`finalize`](Self::finalize) to produce the simulation output.
pub(crate) struct BattleState {
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub is_sortie: bool,
    pub battle_type: BattleType,
    pub friendly_formation_id: i64,
    pub enemy_formation_id: i64,
    pub engagement: super::types::EngagementType,
    // Phase outputs (accumulated)
    pub kouku: Option<BattleKouku>,
    pub opening_attack: Option<BattleOpeningAttack>,
    pub opening_taisen: Option<BattleHougeki>,
    pub hougeki1: Option<BattleHougeki>,
    pub hougeki2: Option<BattleHougeki>,
    pub raigeki: Option<BattleRaigeki>,
    // Protocol flags
    pub stage_flag: [i64; 3],
    pub hourai_flag: [i64; 4],
    pub opening_taisen_flag: i64,
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
            is_sortie,
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
        }
    }

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
