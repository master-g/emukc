//! Expedition gameplay integration tests.

use emukc_db::{
	entity::profile::{expedition, fleet, quest},
	prelude::new_mem_db,
	sea_orm::{
		ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
	},
};
use emukc_gameplay::prelude::*;
use emukc_model::{codex::Codex, kc2::level, prelude::ExpeditionResult};
use emukc_time::chrono::{Duration, Utc};

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
	let model = fleet::Entity::find()
		.filter(fleet::Column::ProfileId.eq(profile_id))
		.filter(fleet::Column::Index.eq(fleet_id))
		.one(context.db())
		.await
		.unwrap()
		.unwrap();

	let mut am = model.into_active_model();
	am.return_time = ActiveValue::Set(Some(Utc::now() - Duration::minutes(1)));
	am.update(context.db()).await.unwrap();
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

	let result = context.complete_expedition(pid, 1).await.unwrap();
	assert_eq!(result.result, ExpeditionResult::Failure);
	assert_eq!(result.admiral_exp, 0);
	assert!(result.resource_reward.is_none());
	assert!(result.item_rewards.is_empty());

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
