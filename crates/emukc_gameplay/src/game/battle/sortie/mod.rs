use emukc_battle::{
    BattleContext, BattleOutcome, BattlePacket, BattleRuntimeShip, BattleSimulation,
    NightBattlePacket,
};

pub(crate) mod orchestrate;
pub(crate) mod response;

pub use orchestrate::{
    pending_battle, run_day_battle, run_night_battle, run_sp_midnight_battle,
    take_day_battle_result,
};
pub use response::{build_day_response, build_night_response};

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

#[derive(Debug, Clone)]
pub struct SortieNightBattleSession {
    #[allow(dead_code)]
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
    use super::super::repository::SortieRepository;
    use super::*;
    use emukc_battle::{BattleShipInput, BattleType, EngagementType, simulate_day};
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
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let store = crate::game::sortie_store::TestSortieStore::new();
        let mut rng = super::super::rng::CryptoRng;
        let simulation = simulate_day(
            &codex,
            BattleContext {
                battle_type: BattleType::Normal,
                is_sortie: true,
                friendly_formation_id: 1,
                enemy_formation_id: 1,
                engagement: EngagementType::SameCourse,
                friend_ships: vec![sample_ship(&codex, 89, 99)],
                enemy_ships: vec![sample_ship(&codex, 412, 99)],
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
}
