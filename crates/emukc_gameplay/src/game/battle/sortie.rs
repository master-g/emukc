#![allow(dead_code)]

use emukc_model::codex::Codex;

use super::core::{
    BattleContext, BattleOutcome, BattlePacket, BattleRuntimeShip, BattleSimulation,
    EngagementType, NightBattlePacket, simulate_day_battle_v1, simulate_night_battle_v1,
};

use crate::game::sortie_store::SortieStore;

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
    pub profile_id: i64,
    pub packet: NightBattlePacket,
    pub outcome: BattleOutcome,
}

pub fn simulate_and_store_sortie_day_battle(
    store: &SortieStore,
    codex: &Codex,
    input: SortieBattleInput,
) -> SortieBattleSession {
    let SortieBattleInput {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        context,
    } = input;
    let simulation = simulate_day_battle_v1(codex, context);
    let session = build_sortie_session(profile_id, deck_id, map_id, cell_id, simulation);
    store.insert_pending_battle(session.profile_id, session.clone());
    session
}

pub fn take_sortie_day_battle_result(
    store: &SortieStore,
    profile_id: i64,
) -> Option<SortieBattleSession> {
    store.take_pending_battle(profile_id)
}

pub fn pending_sortie_battle(store: &SortieStore, profile_id: i64) -> Option<SortieBattleSession> {
    store.get_pending_battle(profile_id)
}

pub fn simulate_and_store_sortie_night_battle(
    store: &SortieStore,
    codex: &Codex,
    profile_id: i64,
    friendly_formation_id: i64,
    enemy_formation_id: i64,
    engagement: EngagementType,
) -> Option<SortieNightBattleSession> {
    let session = store.get_pending_battle(profile_id)?;
    let simulation = simulate_night_battle_v1(
        codex,
        session.friendly.clone(),
        session.enemy.clone(),
        friendly_formation_id,
        enemy_formation_id,
        engagement,
    );
    store.with_pending_battle_mut(profile_id, |s| {
        s.friendly = simulation.friendly.clone();
        s.enemy = simulation.enemy.clone();
        s.outcome = simulation.outcome.clone();
        s.packet.friendly_nowhps = simulation.packet.friendly_nowhps.clone();
        s.packet.enemy_nowhps = simulation.packet.enemy_nowhps.clone();
        s.packet.midnight_flag = 0;
    });

    Some(SortieNightBattleSession {
        profile_id,
        packet: simulation.packet,
        outcome: simulation.outcome,
    })
}

/// Creates and stores a sortie session for a night-start (sp_midnight) battle.
///
/// Unlike normal midnight which continues from a day battle, sp_midnight has
/// no preceding day battle. We construct a minimal day packet (no combat phases)
/// and immediately run the night simulation.
pub fn simulate_and_store_sortie_sp_midnight_battle(
    store: &SortieStore,
    codex: &Codex,
    input: SortieBattleInput,
    enemy_formation_id: i64,
) -> (SortieBattleSession, SortieNightBattleSession) {
    let SortieBattleInput {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        context,
    } = input;

    let friendly_formation_id = context.friendly_formation_id;
    let engagement = context.engagement;
    let friendly: Vec<BattleRuntimeShip> =
        context.friend_ships.into_iter().map(|s| BattleRuntimeShip::new(s, true, true)).collect();
    let enemy: Vec<BattleRuntimeShip> =
        context.enemy_ships.into_iter().map(|s| BattleRuntimeShip::new(s, false, true)).collect();

    // Create a minimal day session to anchor the night battle
    let day_session = SortieBattleSession {
        profile_id,
        deck_id,
        map_id,
        cell_id,
        friendly_ship_ids: friendly.iter().map(|s| s.ship.api_id).collect(),
        enemy_ship_ids: enemy.iter().map(|s| s.ship.api_ship_id).collect(),
        friendly: friendly.clone(),
        enemy: enemy.clone(),
        packet: BattlePacket {
            formation: [friendly_formation_id, enemy_formation_id, engagement.api_id()],
            friendly_nowhps: friendly.iter().map(|s| s.hp()).collect(),
            friendly_maxhps: friendly.iter().map(|s| s.ship.api_maxhp).collect(),
            enemy_nowhps: enemy.iter().map(|s| s.hp()).collect(),
            enemy_maxhps: enemy.iter().map(|s| s.ship.api_maxhp).collect(),
            smoke_type: 0,
            balloon_cell: 0,
            atoll_cell: 0,
            midnight_flag: 1,
            search: [1, 1],
            stage_flag: [0, 0, 0],
            kouku: None,
            opening_taisen_flag: 0,
            opening_taisen: None,
            opening_flag: 0,
            opening_attack: None,
            hourai_flag: [0, 0, 0, 0],
            hougeki1: None,
            hougeki2: None,
            hougeki3: None,
            raigeki: None,
        },
        outcome: BattleOutcome {
            win_rank: "D".to_string(),
            mvp: 0,
            can_midnight: true,
        },
    };
    store.insert_pending_battle(profile_id, day_session.clone());

    // Run night battle using the stored session
    let night = simulate_night_battle_v1(
        codex,
        friendly,
        enemy,
        friendly_formation_id,
        enemy_formation_id,
        engagement,
    );

    // Update the stored day session with night results
    store.with_pending_battle_mut(profile_id, |session| {
        session.friendly = night.friendly.clone();
        session.enemy = night.enemy.clone();
        session.outcome = night.outcome.clone();
        session.packet.friendly_nowhps = night.packet.friendly_nowhps.clone();
        session.packet.enemy_nowhps = night.packet.enemy_nowhps.clone();
        session.packet.midnight_flag = 0;
    });

    let night_session = SortieNightBattleSession {
        profile_id,
        packet: night.packet,
        outcome: night.outcome,
    };

    (day_session, night_session)
}

fn build_sortie_session(
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
    use crate::game::battle::core::{BattleMode, BattleShipInput, BattleType, EngagementType};
    use emukc_model::kc2::level;

    fn sample_ship(codex: &Codex, mst_id: i64, level: i64) -> BattleShipInput {
        let (mut ship, slot_items) = codex.new_ship(mst_id).unwrap();
        let exp_now = level::ship_level_required_exp(level);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = level;
        ship.api_exp = [exp_now, next_exp, 0];
        codex.cal_ship_status(&mut ship, &slot_items).unwrap();
        BattleShipInput {
            ship,
            slot_items,
            effect_list: vec![0],
        }
    }

    #[test]
    fn sortie_session_is_stored_until_result_is_taken() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let store = SortieStore::new();
        let session = simulate_and_store_sortie_day_battle(
            &store,
            &codex,
            SortieBattleInput {
                profile_id: 42,
                deck_id: 1,
                map_id: 11,
                cell_id: 3,
                context: BattleContext {
                    mode: BattleMode::Sortie,
                    battle_type: BattleType::Normal,
                    is_sortie: true,
                    friendly_formation_id: 1,
                    enemy_formation_id: 1,
                    engagement: EngagementType::SameCourse,
                    friend_ships: vec![sample_ship(&codex, 89, 99)],
                    enemy_ships: vec![sample_ship(&codex, 412, 99)],
                    rng_seed: Some(1),
                },
            },
        );

        assert_eq!(session.profile_id, 42);
        assert_eq!(session.map_id, 11);
        assert!(!session.enemy_ship_ids.is_empty());

        let taken = take_sortie_day_battle_result(&store, 42).unwrap();
        assert_eq!(taken.cell_id, 3);
        assert!(take_sortie_day_battle_result(&store, 42).is_none());
    }
}
