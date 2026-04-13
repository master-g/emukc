//! Expedition gameplay integration tests.

use emukc_db::{
    entity::profile::{expedition, fleet, quest, ship::morale_timer},
    prelude::new_mem_db,
    sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    },
};
use emukc_gameplay::prelude::*;
use emukc_model::{codex::Codex, kc2::level, prelude::ExpeditionResult};
use emukc_time::chrono::{DateTime, Duration, Utc};

async fn mock_context() -> (emukc_db::sea_orm::DbConn, Codex) {
    let db = new_mem_db().await.unwrap();
    let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
    (db, codex)
}

async fn new_game_session() -> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
    let context = mock_context().await;

    let account = context.sign_up("test", "1234567").await.unwrap();
    let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
    let session =
        context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

    (context, session)
}

async fn add_ship_with_type(
    context: &(emukc_db::sea_orm::DbConn, Codex),
    profile_id: i64,
    ship_type: i64,
    level_req: i64,
) -> i64 {
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

async fn set_fleet_ships(
    context: &(emukc_db::sea_orm::DbConn, Codex),
    profile_id: i64,
    fleet_id: i64,
    ship_ids: &[i64],
) {
    let mut slots = [-1; 6];
    for (idx, ship_id) in ship_ids.iter().enumerate() {
        slots[idx] = *ship_id;
    }
    context.update_fleet_ships(profile_id, fleet_id, &slots).await.unwrap();
}

async fn make_fleet_ready_for_result(
    context: &(emukc_db::sea_orm::DbConn, Codex),
    profile_id: i64,
    fleet_id: i64,
) {
    set_fleet_return_time(context, profile_id, fleet_id, Utc::now() - Duration::minutes(1)).await;
}

async fn set_fleet_return_time(
    context: &(emukc_db::sea_orm::DbConn, Codex),
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

async fn get_ship_supply(context: &(emukc_db::sea_orm::DbConn, Codex), ship_id: i64) -> (i64, i64) {
    let ship = context.find_ship(ship_id).await.unwrap().unwrap();
    (ship.api_fuel, ship.api_bull)
}

async fn set_ship_supply(
    context: &(emukc_db::sea_orm::DbConn, Codex),
    ship_id: i64,
    fuel: i64,
    ammo: i64,
) {
    let mut ship = context.find_ship(ship_id).await.unwrap().unwrap();
    ship.api_fuel = fuel;
    ship.api_bull = ammo;
    context.update_ship(&ship).await.unwrap();
}

async fn set_ship_condition(
    context: &(emukc_db::sea_orm::DbConn, Codex),
    ship_id: i64,
    condition: i64,
) {
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

#[tokio::test]
async fn expedition_start_validates_fleet_composition() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let cl = add_ship_with_type(&context, pid, 3, 3).await;
    let dd_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let dd_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[cl, dd_1, dd_2]).await;

    let err = context.start_expedition(pid, 1, 10).await.unwrap_err();
    assert!(err.to_string().contains("composition"));

    let cl_2 = add_ship_with_type(&context, pid, 3, 1).await;
    set_fleet_ships(&context, pid, 1, &[cl, cl_2, dd_1]).await;
    let started = context.start_expedition(pid, 1, 10).await.unwrap();
    assert!(started.complete_time > Utc::now());
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
async fn expedition_start_requires_full_supply_even_if_current_stocks_cover_cost() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    let before_1 = get_ship_supply(&context, ship_1).await;
    let before_2 = get_ship_supply(&context, ship_2).await;
    set_ship_supply(&context, ship_1, before_1.0, before_1.1 - 1).await;

    let err = context.start_expedition(pid, 1, 1).await.unwrap_err();
    assert!(err.to_string().contains("ammo"));

    assert_eq!(get_ship_supply(&context, ship_1).await, (before_1.0, before_1.1 - 1));
    assert_eq!(get_ship_supply(&context, ship_2).await, before_2);
    let fleet = context.get_fleet(pid, 1).await.unwrap();
    assert!(fleet.mission.is_none());
}

#[tokio::test]
async fn expedition_start_rejects_when_fuel_is_insufficient_without_partial_changes() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2]).await;

    let before_1 = get_ship_supply(&context, ship_1).await;
    let before_2 = get_ship_supply(&context, ship_2).await;
    set_ship_supply(&context, ship_1, 3, before_1.1).await;

    let err = context.start_expedition(pid, 1, 1).await.unwrap_err();
    assert!(err.to_string().contains("fuel"));

    assert_eq!(get_ship_supply(&context, ship_1).await, (3, before_1.1));
    assert_eq!(get_ship_supply(&context, ship_2).await, before_2);

    let fleet = context.get_fleet(pid, 1).await.unwrap();
    assert!(fleet.mission.is_none());
    let (records, _) = context.get_expeditions(pid).await.unwrap();
    assert!(records.iter().all(|record| record.mission_id != 1));
}

#[tokio::test]
async fn expedition_start_rejects_when_ammo_is_insufficient_without_partial_changes() {
    let (context, session) = new_game_session().await;
    let pid = session.profile.id;

    let ship_1 = add_ship_with_type(&context, pid, 2, 3).await;
    let ship_2 = add_ship_with_type(&context, pid, 2, 1).await;
    let ship_3 = add_ship_with_type(&context, pid, 2, 1).await;
    set_fleet_ships(&context, pid, 1, &[ship_1, ship_2, ship_3]).await;

    let before_1 = get_ship_supply(&context, ship_1).await;
    let before_2 = get_ship_supply(&context, ship_2).await;
    let before_3 = get_ship_supply(&context, ship_3).await;
    set_ship_supply(&context, ship_2, before_2.0, 2).await;

    let err = context.start_expedition(pid, 1, 3).await.unwrap_err();
    assert!(err.to_string().contains("ammo"));

    assert_eq!(get_ship_supply(&context, ship_1).await, before_1);
    assert_eq!(get_ship_supply(&context, ship_2).await, (before_2.0, 2));
    assert_eq!(get_ship_supply(&context, ship_3).await, before_3);

    let fleet = context.get_fleet(pid, 1).await.unwrap();
    assert!(fleet.mission.is_none());
    let (records, _) = context.get_expeditions(pid).await.unwrap();
    assert!(records.iter().all(|record| record.mission_id != 3));
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
