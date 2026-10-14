//! Expedition gameplay integration tests.

use std::sync::Arc;

use emukc_db::{
    entity::profile::{
        expedition, fleet, quest,
        ship::{self, morale_timer},
    },
    prelude::new_mem_db,
    sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    },
};
use emukc_gameplay::prelude::*;
use emukc_model::{codex::Codex, kc2::level, prelude::ExpeditionResult};
use emukc_time::chrono::{DateTime, Duration, Utc};

async fn mock_context() -> Ctx {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    Ctx::new(Arc::new(db), Arc::new(codex))
}

async fn new_game_session() -> (Ctx, StartGameInfo) {
    let context = mock_context().await;

    let account = context.sign_up("test", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

    (context, session)
}

async fn add_ship_with_type(context: &Ctx, profile_id: i64, ship_type: i64, level_req: i64) -> i64 {
    let mst_id = context
        .codex()
        .manifest
        .api_mst_ship
        .iter()
        .find(|ship| {
            ship.api_stype == ship_type && context.codex().ship_extra.contains_key(&ship.api_id)
        })
        .map(|ship| ship.api_id)
        .unwrap();

    let mut ship = context.add_ship(profile_id, mst_id).await.unwrap();
    if ship.api_lv < level_req {
        let exp_now = level::ship_level_required_exp(level_req);
        let (_, next_exp) = level::exp_to_ship_level(exp_now);
        ship.api_lv = level_req;
        ship.api_exp = [exp_now, next_exp, 0];
        context.update_ship(&ship).await.unwrap();
    }

    ship.api_id
}

async fn set_fleet_ships(context: &Ctx, profile_id: i64, fleet_id: i64, ship_ids: &[i64]) {
    let mut slots = [-1; 6];
    for (idx, ship_id) in ship_ids.iter().enumerate() {
        slots[idx] = *ship_id;
    }
    context.update_fleet_ships(profile_id, fleet_id, &slots).await.unwrap();
}

async fn make_fleet_ready_for_result(context: &Ctx, profile_id: i64, fleet_id: i64) {
    set_fleet_return_time(context, profile_id, fleet_id, Utc::now() - Duration::minutes(1)).await;
}

async fn set_fleet_return_time(
    context: &Ctx,
    profile_id: i64,
    fleet_id: i64,
    return_time: DateTime<Utc>,
) {
    let model = fleet::Entity::find()
        .filter(fleet::Column::ProfileId.eq(profile_id))
        .filter(fleet::Column::Index.eq(fleet_id))
        .one(context.db())
        .await
        .unwrap()
        .unwrap();

    let mut am = model.into_active_model();
    am.return_time = ActiveValue::Set(Some(return_time));
    am.update(context.db()).await.unwrap();
}

async fn get_ship_supply(context: &Ctx, ship_id: i64) -> (i64, i64) {
    let ship = context.find_ship(ship_id).await.unwrap().unwrap();
    (ship.api_fuel, ship.api_bull)
}

async fn set_ship_supply(context: &Ctx, ship_id: i64, fuel: i64, ammo: i64) {
    let mut ship = context.find_ship(ship_id).await.unwrap().unwrap();
    ship.api_fuel = fuel;
    ship.api_bull = ammo;
    context.update_ship(&ship).await.unwrap();
}

async fn set_ship_condition(context: &Ctx, ship_id: i64, condition: i64) {
    let mut ship = context.find_ship(ship_id).await.unwrap().unwrap();
    ship.api_cond = condition;
    context.update_ship(&ship).await.unwrap();
}

#[tokio::test]
async fn expedition_result_updates_records_and_quest_progress() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    if !context.get_quest_records(pid).await.unwrap().iter().any(|record| record.quest_id == 401) {
        context.quest_add(pid, 401).await.unwrap();
    }
    context.quest_start(pid, 401).await.unwrap();
    set_ship_condition(&context, ship_1, 43).await;
    set_ship_condition(&context, ship_2, 43).await;

    context.start_expedition(pid, 1, 1).await.unwrap();
    make_fleet_ready_for_result(&context, pid, 1).await;

    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_eq!(result.result, ExpeditionResult::Success);
    assert_eq!(result.mission_id, 1);
    assert!(result.admiral_exp > 0);

    let (records, _) = context.get_expeditions(pid).await.unwrap();
    let mission_record = records.iter().find(|record| record.mission_id == 1).unwrap();
    assert_eq!(mission_record.state, expedition::Status::Completed);

    let quest = context
        .get_quest_records(pid)
        .await
        .unwrap()
        .into_iter()
        .find(|record| record.quest_id == 401)
        .unwrap();
    assert_eq!(quest.progress, quest::progress::Progress::Completed);

    let fleet = context.get_fleet(pid, 1).await.unwrap();
    assert!(fleet.mission.is_none());
}

#[tokio::test]
async fn recalled_expedition_fails_without_advancing_quest() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    if !context.get_quest_records(pid).await.unwrap().iter().any(|record| record.quest_id == 401) {
        context.quest_add(pid, 401).await.unwrap();
    }
    context.quest_start(pid, 401).await.unwrap();

    context.start_expedition(pid, 1, 1).await.unwrap();
    context.recall_expedition(pid, 1).await.unwrap();
    make_fleet_ready_for_result(&context, pid, 1).await;

    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_eq!(result.result, ExpeditionResult::Failure);
    assert_eq!(result.admiral_exp, 0);
    assert!(result.resource_reward.is_none());
    assert!(result.item_rewards.iter().all(Option::is_none));

    let (records, _) = context.get_expeditions(pid).await.unwrap();
    let mission_record = records.iter().find(|record| record.mission_id == 1).unwrap();
    assert_eq!(mission_record.state, expedition::Status::Unfinished);

    let quest = context
        .get_quest_records(pid)
        .await
        .unwrap()
        .into_iter()
        .find(|record| record.quest_id == 401)
        .unwrap();
    assert_eq!(quest.progress, quest::progress::Progress::Empty);
}

/// Fuel, ammo, steel, bauxite and buckets: what a successful expedition can pay.
async fn rewards_held(context: &Ctx, profile_id: i64) -> [i64; 5] {
    let m = context.get_materials(profile_id).await.unwrap();
    [m.fuel, m.ammo, m.steel, m.bauxite, m.bucket]
}

/// Mission 10 asks for two light cruisers: one short of it still sails, and
/// fails on return without paying anything.
#[tokio::test]
async fn an_expedition_short_of_its_composition_sails_and_fails_on_return() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let cl = add_ship_with_type(&context, pid, 3, 3).await;
    let dd_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let dd_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[cl, dd_1, dd_2]).await;
    // Morale that survives the return, so only the composition can fail it.
    for ship in [cl, dd_1, dd_2] {
        set_ship_condition(&context, ship, 43).await;
    }

    let started = context.start_expedition(pid, 1, 10).await.unwrap();
    assert!(started.complete_time > Utc::now());
    let held = rewards_held(&context, pid).await;
    make_fleet_ready_for_result(&context, pid, 1).await;

    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_eq!(result.result, ExpeditionResult::Failure);
    assert!(result.resource_reward.is_none());
    assert_eq!(rewards_held(&context, pid).await, held);

    // A fresh fleet: the first trip left these short of fuel.
    let cl = add_ship_with_type(&context, pid, 3, 3).await;
    let cl_2 = add_ship_with_type(&context, pid, 3, 1).await;
    let dd = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[cl, cl_2, dd]).await;
    // Morale drops on return; 40 would come back fatigued.
    for ship in [cl, cl_2, dd] {
        set_ship_condition(&context, ship, 43).await;
    }
    context.start_expedition(pid, 1, 10).await.unwrap();
    make_fleet_ready_for_result(&context, pid, 1).await;
    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_ne!(result.result, ExpeditionResult::Failure);
}

#[tokio::test]
async fn expedition_start_deducts_ship_fuel() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    let before_1 = get_ship_supply(&context, ship_1).await;
    let before_2 = get_ship_supply(&context, ship_2).await;

    context.start_expedition(pid, 1, 1).await.unwrap();

    let after_1 = get_ship_supply(&context, ship_1).await;
    let after_2 = get_ship_supply(&context, ship_2).await;

    assert_eq!(after_1.0, before_1.0 - 4);
    assert_eq!(after_1.1, before_1.1);
    assert_eq!(after_2.0, before_2.0 - 4);
    assert_eq!(after_2.1, before_2.1);
}

#[tokio::test]
async fn a_ship_not_fully_supplied_sails_and_fails_on_return() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    let before_1 = get_ship_supply(&context, ship_1).await;
    set_ship_supply(&context, ship_1, before_1.0, before_1.1 - 1).await;
    set_ship_condition(&context, ship_1, 43).await;
    set_ship_condition(&context, ship_2, 43).await;

    context.start_expedition(pid, 1, 1).await.unwrap();
    let held = rewards_held(&context, pid).await;
    make_fleet_ready_for_result(&context, pid, 1).await;

    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_eq!(result.result, ExpeditionResult::Failure);
    assert_eq!(rewards_held(&context, pid).await, held);
}

/// Mission 1 costs a destroyer 4 fuel; one with 3 sails and comes back with
/// none (and fails, not being fully supplied).
#[tokio::test]
async fn a_launch_cost_takes_a_short_ship_down_to_zero_fuel() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    let before_1 = get_ship_supply(&context, ship_1).await;
    let before_2 = get_ship_supply(&context, ship_2).await;
    set_ship_supply(&context, ship_1, 3, before_1.1).await;
    set_ship_condition(&context, ship_1, 43).await;
    set_ship_condition(&context, ship_2, 43).await;

    context.start_expedition(pid, 1, 1).await.unwrap();

    assert_eq!(get_ship_supply(&context, ship_1).await, (0, before_1.1));
    assert_eq!(get_ship_supply(&context, ship_2).await, (before_2.0 - 4, before_2.1));
    make_fleet_ready_for_result(&context, pid, 1).await;
    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_eq!(result.result, ExpeditionResult::Failure);
}

/// Mission 3 costs a fifth of a ship's ammo, more than 2.
#[tokio::test]
async fn a_launch_cost_takes_a_short_ship_down_to_zero_ammo() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 3).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_3 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2, ship_3]).await;

    let before_2 = get_ship_supply(&context, ship_2).await;
    set_ship_supply(&context, ship_2, before_2.0, 2).await;

    context.start_expedition(pid, 1, 3).await.unwrap();

    assert_eq!(get_ship_supply(&context, ship_2).await.1, 0);
    let fleet = context.get_fleet(pid, 1).await.unwrap();
    assert!(fleet.mission.is_some());
}

#[tokio::test]
async fn expedition_recall_and_result_do_not_restore_start_supply_costs() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    let before_1 = get_ship_supply(&context, ship_1).await;
    let before_2 = get_ship_supply(&context, ship_2).await;

    context.start_expedition(pid, 1, 1).await.unwrap();
    let started_1 = get_ship_supply(&context, ship_1).await;
    let started_2 = get_ship_supply(&context, ship_2).await;
    set_fleet_return_time(&context, pid, 1, Utc::now() + Duration::minutes(12)).await;
    context.recall_expedition(pid, 1).await.unwrap();
    make_fleet_ready_for_result(&context, pid, 1).await;
    context.complete_expedition(pid, 1).await.unwrap();

    assert_eq!(started_1.0, before_1.0 - 4);
    assert_eq!(started_2.0, before_2.0 - 4);
    assert_eq!(get_ship_supply(&context, ship_1).await, started_1);
    assert_eq!(get_ship_supply(&context, ship_2).await, started_2);
}

#[tokio::test]
async fn expedition_recall_sets_shortened_return_time_and_blocks_early_result() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    context.start_expedition(pid, 1, 1).await.unwrap();
    set_fleet_return_time(&context, pid, 1, Utc::now() + Duration::minutes(12)).await;

    let before_recall = Utc::now();
    context.recall_expedition(pid, 1).await.unwrap();

    let fleet = context.get_fleet(pid, 1).await.unwrap();
    let mission = fleet.mission.unwrap();
    assert_eq!(mission.status as i64, 3);
    let seconds_until_return = (mission.return_time.unwrap() - before_recall).num_seconds();
    assert!((55..=65).contains(&seconds_until_return));

    let err = context.complete_expedition(pid, 1).await.unwrap_err();
    assert!(err.to_string().contains("not ready yet"));
}

#[tokio::test]
async fn expedition_failure_from_fatigue_still_grants_failure_exp() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;
    set_ship_condition(&context, ship_1, 42).await;
    set_ship_condition(&context, ship_2, 42).await;

    context.start_expedition(pid, 1, 1).await.unwrap();
    make_fleet_ready_for_result(&context, pid, 1).await;

    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_eq!(result.result, ExpeditionResult::Failure);
    assert_eq!(result.admiral_exp, 3);
    assert_eq!(result.ship_exp, vec![15, 10]);
    assert!(result.resource_reward.is_none());
    assert!(result.item_rewards.iter().all(Option::is_none));

    let (records, _) = context.get_expeditions(pid).await.unwrap();
    let mission_record = records.iter().find(|record| record.mission_id == 1).unwrap();
    assert_eq!(mission_record.state, expedition::Status::Unfinished);
    assert_eq!(context.find_ship(ship_1).await.unwrap().unwrap().api_cond, 39);
    assert_eq!(context.find_ship(ship_2).await.unwrap().unwrap().api_cond, 39);
}

#[tokio::test]
async fn expedition_exp_pins_unmarried_ship_at_level_99() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 98).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;
    set_ship_condition(&context, ship_1, 43).await;
    set_ship_condition(&context, ship_2, 43).await;

    // Park the flagship one exp point below the unmarried level cap.
    let model = ship::Entity::find_by_id(ship_1).one(context.db()).await.unwrap().unwrap();
    let mut am = model.into_active_model();
    am.level = ActiveValue::Set(98);
    am.exp_now = ActiveValue::Set(level::ship_level_required_exp(99) - 1);
    am.exp_next = ActiveValue::Set(level::ship_level_required_exp(99));
    am.exp_progress = ActiveValue::Set(99);
    am.married = ActiveValue::Set(false);
    am.update(context.db()).await.unwrap();

    context.start_expedition(pid, 1, 1).await.unwrap();
    make_fleet_ready_for_result(&context, pid, 1).await;
    context.complete_expedition(pid, 1).await.unwrap();

    let after = ship::Entity::find_by_id(ship_1).one(context.db()).await.unwrap().unwrap();
    assert_eq!(after.level, 99);
    assert_eq!(after.exp_now, level::ship_level_required_exp(99));
    assert_eq!(after.exp_next, 0);
    assert_eq!(after.exp_progress, 0);
}

#[tokio::test]
async fn morale_recovers_three_points_every_three_minutes_up_to_49() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_id = add_ship_with_type(&context, pid, 2, 1).await;
    set_ship_condition(&context, ship_id, 40).await;

    let timer = morale_timer::Entity::find_by_id(pid).one(context.db()).await.unwrap().unwrap();
    let mut am = timer.into_active_model();
    am.last_time_regen = ActiveValue::Set(Some(Utc::now() - Duration::minutes(6)));
    am.update(context.db()).await.unwrap();

    let ship = context.find_ship(ship_id).await.unwrap().unwrap();
    assert_eq!(ship.api_cond, 40);

    let ships = context.get_ships(pid).await.unwrap();
    let ship = ships.into_iter().find(|ship| ship.api_id == ship_id).unwrap();
    assert_eq!(ship.api_cond, 46);
}

#[tokio::test]
async fn expedition_type1_great_success_applies_reward_multipliers() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let mut ship_ids = Vec::new();
    for idx in 0..6 {
        let level_req = if idx == 0 {
            6
        } else {
            1
        };
        let ship_id = add_ship_with_type(&context, pid, 2, level_req).await;
        set_ship_condition(&context, ship_id, 50).await;
        ship_ids.push(ship_id);
    }
    set_fleet_ships(&context, pid, 1, &ship_ids).await;

    context.start_expedition(pid, 1, 8).await.unwrap();
    for ship_id in &ship_ids {
        set_ship_condition(&context, *ship_id, 43).await;
    }
    make_fleet_ready_for_result(&context, pid, 1).await;

    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_eq!(result.result, ExpeditionResult::GreatSuccess);
    assert_eq!(result.resource_reward, Some([75, 150, 75, 75]));
    assert_eq!(result.admiral_exp, 240);
    assert_eq!(result.ship_exp.len(), 6);
    assert_eq!(result.ship_exp[0], 420);
    assert!(result.ship_exp[1..].iter().all(|exp| *exp == 280));
    assert_eq!(result.item_rewards[1].as_ref().map(|reward| reward.item_id), Some(3));
    assert_eq!(result.item_rewards[1].as_ref().map(|reward| reward.count), Some(1));
    assert!(
        result.item_rewards[0]
            .as_ref()
            .is_none_or(|reward| reward.item_id == 2 && (1..=2).contains(&reward.count))
    );
}

#[tokio::test]
async fn initial_ship_morale_does_not_jump_on_first_read() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_id = add_ship_with_type(&context, pid, 2, 1).await;

    let ship = context.find_ship(ship_id).await.unwrap().unwrap();
    assert_eq!(ship.api_cond, 40);

    let ship = context
        .get_ships(pid)
        .await
        .unwrap()
        .into_iter()
        .find(|ship| ship.api_id == ship_id)
        .unwrap();
    assert_eq!(ship.api_cond, 40);
}

#[tokio::test]
async fn expedition_launch_snapshot_is_persisted_on_fleet_and_cleared_after_result() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let mut ship_ids = Vec::new();
    for idx in 0..6 {
        let level_req = if idx == 0 {
            6
        } else {
            1
        };
        let ship_id = add_ship_with_type(&context, pid, 2, level_req).await;
        set_ship_condition(&context, ship_id, 50).await;
        ship_ids.push(ship_id);
    }
    set_fleet_ships(&context, pid, 1, &ship_ids).await;

    context.start_expedition(pid, 1, 8).await.unwrap();

    let fleet_row = fleet::Entity::find()
        .filter(fleet::Column::ProfileId.eq(pid))
        .filter(fleet::Column::Index.eq(1))
        .one(context.db())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fleet_row.launch_fleet_ship_count, 6);
    assert_eq!(fleet_row.launch_sparkled_ship_count, 6);
    assert_eq!(fleet_row.launch_flagship_level, 6);

    make_fleet_ready_for_result(&context, pid, 1).await;
    context.complete_expedition(pid, 1).await.unwrap();

    let fleet_row = fleet::Entity::find()
        .filter(fleet::Column::ProfileId.eq(pid))
        .filter(fleet::Column::Index.eq(1))
        .one(context.db())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fleet_row.launch_fleet_ship_count, 0);
    assert_eq!(fleet_row.launch_sparkled_ship_count, 0);
    assert_eq!(fleet_row.launch_flagship_level, 0);
    assert_eq!(fleet_row.launch_drum_ship_count, 0);
    assert_eq!(fleet_row.launch_total_drums, 0);
}

/// A2 (mission 101): four destroyers, flagship Lv20, firepower 50, AA 70 and
/// ASW 180 in total. Each ship gets a quarter of `[firepower, aa, asw]` (rounded
/// up) and morale that survives the return.
async fn a2_fleet(context: &Ctx, profile_id: i64, totals: [i64; 3]) -> Vec<i64> {
    let mut ids = Vec::new();
    for idx in 0..4 {
        let id = add_ship_with_type(
            context,
            profile_id,
            2,
            if idx == 0 {
                20
            } else {
                1
            },
        )
        .await;
        set_stats(context, id, totals.map(|total| (total + 3) / 4)).await;
        ids.push(id);
    }
    set_fleet_ships(context, profile_id, 1, &ids).await;
    ids
}

/// Set a ship's current firepower, AA and ASW, and morale that survives a
/// return, straight in the row: `update_ship` rebuilds stats from the master,
/// its level and its equipment.
async fn set_stats(context: &Ctx, ship_id: i64, [firepower, aa, asw]: [i64; 3]) {
    let row = ship::Entity::find_by_id(ship_id).one(context.db()).await.unwrap().unwrap();
    let mut am = row.into_active_model();
    am.condition = ActiveValue::Set(43);
    am.firepower_now = ActiveValue::Set(firepower);
    am.aa_now = ActiveValue::Set(aa);
    am.asw_now = ActiveValue::Set(asw);
    am.update(context.db()).await.unwrap();
}

async fn a2_result(context: &Ctx, profile_id: i64) -> ExpeditionResult {
    context.start_expedition(profile_id, 1, 101).await.unwrap();
    make_fleet_ready_for_result(context, profile_id, 1).await;
    context.complete_expedition(profile_id, 1).await.unwrap().result
}

/// kcanotify's `total-fp` is anti-air: A2 asks for 70 of it (#116).
#[tokio::test]
async fn a2_needs_seventy_anti_air() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    a2_fleet(&context, pid, [52, 68, 180]).await;
    assert_eq!(a2_result(&context, pid).await, ExpeditionResult::Failure);

    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    a2_fleet(&context, pid, [52, 72, 180]).await;
    assert_ne!(a2_result(&context, pid).await, ExpeditionResult::Failure);
}

/// Short of A2's ASW, the fleet sails, fails on return, earns nothing and does
/// not advance 402 (#118).
#[tokio::test]
async fn a2_short_of_asw_sails_and_fails_without_quest_progress() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    context.quest_add(pid, 402).await.unwrap();
    context.quest_start(pid, 402).await.unwrap();
    a2_fleet(&context, pid, [52, 72, 176]).await;

    context.start_expedition(pid, 1, 101).await.unwrap();
    let held = rewards_held(&context, pid).await;
    make_fleet_ready_for_result(&context, pid, 1).await;
    let result = context.complete_expedition(pid, 1).await.unwrap();

    assert_eq!(result.result, ExpeditionResult::Failure);
    assert_eq!(rewards_held(&context, pid).await, held);
    let record = context
        .get_quest_records(pid)
        .await
        .unwrap()
        .into_iter()
        .find(|record| record.quest_id == 402)
        .unwrap();
    assert_eq!(record.requirements, serde_json::json!([{"Expedition": [{"times": 3}]}]));
}

/// The verdict is taken when the fleet leaves: what changes while it is away
/// does not move it either way (#118).
#[tokio::test]
async fn changes_while_away_do_not_change_the_result() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    let ships = a2_fleet(&context, pid, [52, 72, 176]).await;
    context.start_expedition(pid, 1, 101).await.unwrap();
    set_stats(&context, ships[0], [13, 18, 60]).await;
    make_fleet_ready_for_result(&context, pid, 1).await;
    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_eq!(result.result, ExpeditionResult::Failure, "short at launch");

    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    let ships = a2_fleet(&context, pid, [52, 72, 180]).await;
    context.start_expedition(pid, 1, 101).await.unwrap();
    set_stats(&context, ships[0], [13, 18, 0]).await;
    make_fleet_ready_for_result(&context, pid, 1).await;
    let result = context.complete_expedition(pid, 1).await.unwrap();
    assert_ne!(result.result, ExpeditionResult::Failure, "met at launch");
}

/// A fleet of these masters with every total and level requirement met, so
/// only the flagship and composition checks can fail it.
async fn qualified_fleet(context: &Ctx, profile_id: i64, mst_ids: &[i64]) {
    let mut ids = Vec::new();
    for &mst_id in mst_ids {
        let id = context.add_ship(profile_id, mst_id).await.unwrap().api_id;
        let row = ship::Entity::find_by_id(id).one(context.db()).await.unwrap().unwrap();
        let mut am = row.into_active_model();
        am.level = ActiveValue::Set(60);
        am.condition = ActiveValue::Set(43);
        am.firepower_now = ActiveValue::Set(100);
        am.aa_now = ActiveValue::Set(100);
        am.asw_now = ActiveValue::Set(100);
        am.los_now = ActiveValue::Set(100);
        am.update(context.db()).await.unwrap();
        ids.push(id);
    }
    set_fleet_ships(context, profile_id, 1, &ids).await;
}

/// Whether a qualified fleet of `mst_ids` comes back from `mission_id` with
/// anything but failure.
async fn succeeds(mission_id: i64, mst_ids: &[i64]) -> bool {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;
    qualified_fleet(&context, pid, mst_ids).await;
    context.start_expedition(pid, 1, mission_id).await.unwrap();
    make_fleet_ready_for_result(&context, pid, 1).await;
    context.complete_expedition(pid, 1).await.unwrap().result != ExpeditionResult::Failure
}

/// 大鷹, an escort carrier (stype 7, and 27 in expedition rules).
const TAIYOU: i64 = 526;
/// 鳳翔, a light carrier that is not an escort carrier.
const HOUSHOU: i64 = 89;
const MUTSUKI: i64 = 1;
const TENRYUU: i64 = 51;
const ETOROFU: i64 = 524;

/// 43's `flag-cond` is "7/27": a light or escort carrier, not a destroyer (#117).
#[tokio::test]
async fn expedition_43_needs_a_carrier_flagship() {
    let escorts = [TENRYUU, MUTSUKI, MUTSUKI, MUTSUKI, MUTSUKI];
    assert!(succeeds(43, &[[HOUSHOU].as_slice(), &escorts].concat()).await);
    assert!(succeeds(43, &[[TAIYOU].as_slice(), &escorts].concat()).await);
    // Four destroyers and a light cruiser still, but a destroyer leads.
    assert!(!succeeds(43, &[MUTSUKI, TENRYUU, HOUSHOU, MUTSUKI, MUTSUKI, MUTSUKI]).await);
}

/// A4's "27-1|2-2" branch: an escort carrier and two destroyers (#117).
#[tokio::test]
async fn a4_takes_an_escort_carrier_with_two_destroyers() {
    assert!(succeeds(103, &[TAIYOU, MUTSUKI, MUTSUKI, ETOROFU, ETOROFU]).await);
    // 鳳翔 is no escort carrier, and nothing else here makes A4.
    assert!(!succeeds(103, &[HOUSHOU, MUTSUKI, MUTSUKI, ETOROFU, ETOROFU]).await);
}

/// 45 takes an escort or a light carrier with four destroyers or escorts.
#[tokio::test]
async fn expedition_45_takes_an_escort_or_light_carrier() {
    assert!(succeeds(45, &[TAIYOU, MUTSUKI, MUTSUKI, ETOROFU, ETOROFU]).await);
    assert!(succeeds(45, &[HOUSHOU, MUTSUKI, MUTSUKI, ETOROFU, ETOROFU]).await);
    assert!(!succeeds(45, &[MUTSUKI, HOUSHOU, MUTSUKI, ETOROFU, ETOROFU]).await);
}

/// The rebuilt codex reads kcanotify as #116 and #117 say: `total-fp` is AA,
/// "7/27" is two flagship types, and the escort carriers are listed.
#[tokio::test]
async fn the_codex_reads_kcanotify_requirements() {
    let context = mock_context().await;
    let codex = context.codex();
    let requirements = |mission_id: i64| &codex.expedition_conditions[&mission_id].requirements;

    let b1 = requirements(110);
    assert_eq!((b1.total_firepower, b1.total_aa), (None, Some(200)));
    let a2 = requirements(101);
    assert_eq!((a2.total_firepower, a2.total_aa), (Some(50), Some(70)));
    assert_eq!(requirements(43).flagship_types, vec![7, 27]);
    assert_eq!(requirements(45).flagship_types, vec![7, 27]);
    assert!(codex.escort_carriers.contains(&TAIYOU));
    assert!(!codex.escort_carriers.contains(&HOUSHOU));
}
