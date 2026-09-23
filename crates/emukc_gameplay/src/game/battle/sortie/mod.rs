use emukc_battle::{
    BattleContext, BattleOutcome, BattlePacket, BattleRuntimeShip, BattleSimulation,
    NightBattlePacket, NightBattleSimulation,
};

pub(crate) mod orchestrate;

pub use orchestrate::{
    pending_battle, run_day_battle, run_night_battle, run_sp_midnight_battle,
    take_day_battle_result,
};

#[derive(Debug, Clone)]
pub struct SortieBattleInput {
    pub profile_id: i64,
    pub deck_id: i64,
    pub map_id: i64,
    pub cell_id: i64,
    pub context: BattleContext,
}

#[derive(Debug, Clone)]
pub struct SortieBattleSession {
    pub profile_id: i64,
    pub deck_id: i64,
    pub map_id: i64,
    pub cell_id: i64,
    pub friendly_ship_ids: Vec<i64>,
    pub enemy_ship_ids: Vec<i64>,
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub packet: BattlePacket,
    pub outcome: BattleOutcome,
}

impl SortieBattleSession {
    /// How many ships sit a night battle out: 第1艦隊 of a combined fleet, none
    /// otherwise. The ships carry their own deck tag, so nothing stores this.
    fn night_start(&self) -> usize {
        self.friendly.iter().take_while(|ship| ship.is_main_deck()).count()
    }

    /// Where 第2艦隊 begins in `friendly`, or `None` for a single fleet.
    pub fn escort_start(&self) -> Option<usize> {
        Some(self.night_start()).filter(|&start| start > 0)
    }

    /// 第1艦隊 of a combined fleet; empty for a single fleet.
    pub fn main_deck(&self) -> &[BattleRuntimeShip] {
        &self.friendly[..self.night_start()]
    }

    /// The ships that fight at night: 第2艦隊 alone for a combined fleet, the
    /// whole fleet otherwise.
    pub fn night_fleet(&self) -> &[BattleRuntimeShip] {
        &self.friendly[self.night_start()..]
    }

    /// Take in a night battle fought by [`night_fleet`](Self::night_fleet).
    ///
    /// `friendly` and `packet.friendly_nowhps` stay one contiguous vector, 第1艦隊
    /// first and untouched, with the night fleet's state after it.
    pub fn absorb_night(&mut self, night: &NightBattleSimulation) {
        let kept = self.night_start();
        self.friendly.truncate(kept);
        self.friendly.extend(night.friendly.iter().cloned());
        self.enemy = night.enemy.clone();
        self.outcome = night.outcome.clone();
        self.packet.friendly_nowhps.truncate(kept);
        self.packet.friendly_nowhps.extend(night.packet.friendly_nowhps.iter().copied());
        self.packet.enemy_nowhps = night.packet.enemy_nowhps.clone();
        self.packet.midnight_flag = 0;
    }
}

#[derive(Debug, Clone)]
pub struct SortieNightBattleSession {
    #[cfg_attr(not(test), expect(dead_code))]
    pub profile_id: i64,
    pub packet: NightBattlePacket,
    pub outcome: BattleOutcome,
}

pub(crate) fn build_sortie_session(
    profile_id: i64,
    deck_id: i64,
    map_id: i64,
    cell_id: i64,
    simulation: BattleSimulation,
) -> SortieBattleSession {
    SortieBattleSession {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        friendly_ship_ids: simulation.friendly.iter().map(|ship| ship.ship.api_id).collect(),
        enemy_ship_ids: simulation.enemy.iter().map(|ship| ship.ship.api_ship_id).collect(),
        friendly: simulation.friendly,
        enemy: simulation.enemy,
        packet: simulation.packet,
        outcome: simulation.outcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_battle::{BattleShipInput, BattleType, EngagementType, execute_day};
    use emukc_model::{codex::Codex, kc2::level};

    fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
        let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
        let exp_now = level::ship_level_required_exp(level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = level;
        ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut ship, &slot_items, false).unwrap();
        BattleShipInput {
            ship,
            slot_items,
            effect_list: vec![0],
            married: false,
        }
    }

    #[test]
    fn sortie_session_is_stored_until_result_is_taken() {
        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        codex.game_cfg.god_mode = false;
        codex.game_cfg.one_hit_kill = false;
        let store = crate::game::sortie_store::SortieStore::new();
        let mut rng = super::super::rng::ProductionRng;
        let simulation = execute_day(
            &codex,
            BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![sample_ship(&codex, 89, 99)],
                enemy_ships: vec![sample_ship(&codex, 412, 99)],
                combined: None,
            },
            &mut rng,
        );
        let session = build_sortie_session(42, 1, 11, 3, simulation);
        store.insert_pending_battle(42, session.clone());

        assert_eq!(session.profile_id, 42);
        assert_eq!(session.map_id, 11);
        assert!(!session.enemy_ship_ids.is_empty());

        let taken = take_day_battle_result(&store, 42).unwrap();
        assert_eq!(taken.cell_id, 3);
        assert!(take_day_battle_result(&store, 42).is_none());
    }

    /// The deck boundary comes from the ships' own tags: a single fleet has no
    /// escort segment and fights at night whole, a combined one sends 第2艦隊.
    #[test]
    fn session_splits_decks_by_the_ships_own_tags() {
        use emukc_battle::{CombinedSetup, CombinedType};

        let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        codex.game_cfg.god_mode = false;
        codex.game_cfg.one_hit_kill = false;
        let session = |combined: Option<CombinedSetup>| {
            let simulation = execute_day(
                &codex,
                BattleContext {
                    battle_type: BattleType::Normal,
                    is_sortie: true,
                    friendly_formation_id: if combined.is_some() {
                        11
                    } else {
                        1
                    },
                    enemy_formation_id: 1,
                    engagement: EngagementType::SameCourse,
                    friend_ships: vec![sample_ship(&codex, 89, 99); 2],
                    enemy_ships: vec![sample_ship(&codex, 412, 99)],
                    combined,
                },
                &mut super::super::rng::ProductionRng,
            );
            build_sortie_session(42, 1, 11, 3, simulation)
        };

        let single = session(None);
        assert_eq!(single.escort_start(), None);
        assert!(single.main_deck().is_empty());
        assert_eq!(single.night_fleet().len(), 2);

        let combined = session(Some(CombinedSetup {
            combined_type: CombinedType::CarrierTaskForce,
            escort_ships: vec![sample_ship(&codex, 89, 99); 3],
        }));
        assert_eq!(combined.escort_start(), Some(2));
        assert_eq!(combined.main_deck().len(), 2);
        assert!(combined.night_fleet().iter().all(BattleRuntimeShip::is_escort_deck));
    }
}
