//! Practice battle integration tests.

use std::sync::atomic::{AtomicI64, Ordering};

use emukc_db::{
	entity::profile::{self, quest, ship},
	prelude::new_mem_db,
	sea_orm::{
		ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
	},
};
use emukc_gameplay::prelude::*;
use emukc_model::{
	codex::Codex,
	kc2::{KcShipType, level},
	thirdparty::{Kc3rdQuestCondition, Kc3rdQuestRequirement},
};
use emukc_time::chrono::Utc;

static PROFILE_ID_BUMP: AtomicI64 = AtomicI64::new(0);

async fn mock_context() -> (emukc_db::sea_orm::DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
	(db, codex)
}

async fn new_game_session() -> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
	let context = mock_context().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let extra_profiles = PROFILE_ID_BUMP.fetch_add(1, Ordering::Relaxed);
	for idx in 0..extra_profiles {
		let name = format!("warmup-practice-{extra_profiles}-{idx}");
		context.new_profile(&account.access_token.token, &name).await.unwrap();
	}
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, session)
}

fn first_ship_mst_by_type(codex: &Codex, ship_type: KcShipType) -> i64 {
	codex
		.manifest
		.api_mst_ship
		.iter()
		.find(|mst| KcShipType::n(mst.api_stype) == Some(ship_type))
		.map(|mst| mst.api_id)
		.unwrap()
}

async fn ensure_started_quest(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	profile_id: i64,
	quest_id: i64,
) {
	if !context
		.get_quest_records(profile_id)
		.await
		.unwrap()
		.iter()
		.any(|record| record.quest_id == quest_id)
	{
		let quest_manifest = context.1.quest.get(&quest_id).unwrap();
		let (requirements, requirement_type) = match &quest_manifest.requirements {
			Kc3rdQuestRequirement::And(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::And)
			}
			Kc3rdQuestRequirement::OneOf(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::OneOf)
			}
			Kc3rdQuestRequirement::Sequential(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::Sequential)
			}
		};

		quest::progress::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			quest_id: ActiveValue::Set(quest_id),
			status: ActiveValue::Set(quest::progress::Status::Idle),
			progress: ActiveValue::Set(quest::progress::Progress::Empty),
			period: ActiveValue::Set(quest_manifest.period.into()),
			start_since: ActiveValue::Set(Utc::now()),
			requirements: ActiveValue::Set(serde_json::to_value(requirements).unwrap()),
			requirement_type: ActiveValue::Set(requirement_type),
		}
		.insert(&context.0)
		.await
		.unwrap();
	}
	context.quest_start(profile_id, quest_id).await.unwrap();
}

async fn quest_progress_of(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	profile_id: i64,
	quest_id: i64,
) -> quest::progress::Progress {
	context
		.get_quest_records(profile_id)
		.await
		.unwrap()
		.into_iter()
		.find(|record| record.quest_id == quest_id)
		.unwrap()
		.progress
}

async fn raw_quest_record(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	profile_id: i64,
	quest_id: i64,
) -> quest::progress::Model {
	quest::progress::Entity::find()
		.filter(quest::progress::Column::ProfileId.eq(profile_id))
		.filter(quest::progress::Column::QuestId.eq(quest_id))
		.one(&context.0)
		.await
		.unwrap()
		.unwrap()
}

async fn ensure_idle_quest(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	profile_id: i64,
	quest_id: i64,
) {
	if !context
		.get_quest_records(profile_id)
		.await
		.unwrap()
		.iter()
		.any(|record| record.quest_id == quest_id)
	{
		let quest_manifest = context.1.quest.get(&quest_id).unwrap();
		let (requirements, requirement_type) = match &quest_manifest.requirements {
			Kc3rdQuestRequirement::And(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::And)
			}
			Kc3rdQuestRequirement::OneOf(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::OneOf)
			}
			Kc3rdQuestRequirement::Sequential(conditions) => {
				(conditions.clone(), quest::progress::RequirementType::Sequential)
			}
		};

		quest::progress::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			quest_id: ActiveValue::Set(quest_id),
			status: ActiveValue::Set(quest::progress::Status::Idle),
			progress: ActiveValue::Set(quest::progress::Progress::Empty),
			period: ActiveValue::Set(quest_manifest.period.into()),
			start_since: ActiveValue::Set(Utc::now()),
			requirements: ActiveValue::Set(serde_json::to_value(requirements).unwrap()),
			requirement_type: ActiveValue::Set(requirement_type),
		}
		.insert(&context.0)
		.await
		.unwrap();
	}
}

async fn exercise_times_remaining(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	profile_id: i64,
	quest_id: i64,
) -> i64 {
	let quest = context
		.get_quest_records(profile_id)
		.await
		.unwrap()
		.into_iter()
		.find(|record| record.quest_id == quest_id)
		.unwrap();
	let conditions: Vec<Kc3rdQuestCondition> = serde_json::from_value(quest.requirements).unwrap();
	conditions
		.into_iter()
		.find_map(|condition| match condition {
			Kc3rdQuestCondition::Exercise(exercise) => Some(exercise.times),
			_ => None,
		})
		.unwrap()
}

async fn set_ship_level(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	ship_id: i64,
	level_req: i64,
) {
	let mut ship = context.find_ship(ship_id).await.unwrap().unwrap();
	let exp_now = level::ship_level_required_exp(level_req);
	let (_, next_exp) = level::exp_to_ship_level(exp_now);
	ship.api_lv = level_req;
	ship.api_exp = [exp_now, next_exp, 0];
	context.update_ship(&ship).await.unwrap();
}

async fn add_ships_with_type(
	context: &(emukc_db::sea_orm::DbConn, Codex),
	profile_id: i64,
	ship_type: KcShipType,
	count: usize,
	level_req: i64,
) -> Vec<i64> {
	let mst_id = first_ship_mst_by_type(&context.1, ship_type);
	let mut ship_ids = Vec::with_capacity(count);
	for _ in 0..count {
		let ship = context.add_ship(profile_id, mst_id).await.unwrap();
		set_ship_level(context, ship.api_id, level_req).await;
		ship_ids.push(ship.api_id);
	}
	ship_ids
}

#[tokio::test]
async fn practice_battle_and_result_flow_updates_rival_status() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	let before_profile = profile::Entity::find_by_id(pid).one(&context.0).await.unwrap().unwrap();
	let before_ship = ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();

	let mut damaged = before_ship.clone().into_active_model();
	damaged.hp_now = ActiveValue::Set((before_ship.hp_now - 3).max(1));
	damaged.exp_now = ActiveValue::Set(before_ship.exp_next - 1);
	damaged.exp_progress = ActiveValue::Set(99);
	damaged.update(&context.0).await.unwrap();
	let configured_ship =
		ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;

	let battle = context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	assert_eq!(battle.api_deck_id, 1);
	assert_eq!(battle.api_formation, [1, 1, 1]);
	assert!(!battle.api_ship_ke.is_empty());
	assert_eq!(battle.api_f_nowhps[0], (before_ship.hp_now - 3).max(1));

	let result = context.practice_battle_result(pid).await.unwrap();
	assert!(!result.api_win_rank.is_empty());
	assert_eq!(result.api_enemy_info.api_level, rivals.rivals[0].level);

	let rival = context.get_practice_rival_details(pid, enemy_id).await.unwrap();
	assert_ne!(rival.status as i64, 0);

	let after_profile = profile::Entity::find_by_id(pid).one(&context.0).await.unwrap().unwrap();
	let after_ship = ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();
	assert_eq!(after_profile.experience, before_profile.experience + result.api_get_exp);
	assert_eq!(after_ship.exp_now, configured_ship.exp_now + result.api_get_ship_exp[1]);
	assert!(after_ship.level > configured_ship.level);
	assert_eq!(result.api_get_exp_lvup[0][0], configured_ship.exp_now);
	assert!(result.api_get_exp_lvup[0].len() >= 3);
}

#[tokio::test]
async fn practice_battle_result_consumes_resources_and_planes_for_carrier() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let carrier_mst = first_ship_mst_by_type(&context.1, KcShipType::CVL);

	let carrier = context.add_ship(pid, carrier_mst).await.unwrap();
	context.update_fleet_ships(pid, 1, &[carrier.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	let before_ship =
		ship::Entity::find_by_id(carrier.api_id).one(&context.0).await.unwrap().unwrap();
	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;

	context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	context.practice_battle_result(pid).await.unwrap();

	let after_ship =
		ship::Entity::find_by_id(carrier.api_id).one(&context.0).await.unwrap().unwrap();
	assert!(after_ship.fuel < before_ship.fuel);
	assert!(after_ship.ammo < before_ship.ammo);
	let before_onslot = before_ship.onslot_1
		+ before_ship.onslot_2
		+ before_ship.onslot_3
		+ before_ship.onslot_4
		+ before_ship.onslot_5;
	let after_onslot = after_ship.onslot_1
		+ after_ship.onslot_2
		+ after_ship.onslot_3
		+ after_ship.onslot_4
		+ after_ship.onslot_5;
	assert!(after_onslot <= before_onslot);
}

#[tokio::test]
async fn practice_battle_result_completes_intro_exercise_quest() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 301;

	let ships = add_ships_with_type(&context, pid, KcShipType::BB, 4, 99).await;
	context
		.update_fleet_ships(pid, 1, &[ships[0], ships[1], ships[2], ships[3], -1, -1])
		.await
		.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;

	context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	context.practice_battle_result(pid).await.unwrap();

	assert_eq!(
		quest_progress_of(&context, pid, quest_id).await,
		quest::progress::Progress::Completed
	);
}

#[tokio::test]
async fn practice_battle_result_completes_three_win_exercise_quest_after_three_battles() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 303;

	let ships = add_ships_with_type(&context, pid, KcShipType::BB, 6, 99).await;
	context
		.update_fleet_ships(pid, 1, &[ships[0], ships[1], ships[2], ships[3], ships[4], ships[5]])
		.await
		.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	for _ in 0..3 {
		let rivals = context.get_practice_rivals(pid).await.unwrap();
		let enemy_id = rivals.rivals[0].id;
		context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
		let result = context.practice_battle_result(pid).await.unwrap();
		assert!(matches!(result.api_win_rank.as_str(), "S" | "A" | "B"));
	}

	assert_eq!(
		quest_progress_of(&context, pid, quest_id).await,
		quest::progress::Progress::Completed
	);
}

#[tokio::test]
async fn practice_battle_result_decrements_ranked_exercise_quest() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 304;

	let ships = add_ships_with_type(&context, pid, KcShipType::BB, 6, 99).await;
	context
		.update_fleet_ships(pid, 1, &[ships[0], ships[1], ships[2], ships[3], ships[4], ships[5]])
		.await
		.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;

	context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	let result = context.practice_battle_result(pid).await.unwrap();

	assert!(matches!(result.api_win_rank.as_str(), "S" | "A" | "B"));
	assert_eq!(exercise_times_remaining(&context, pid, quest_id).await, 4);
}

#[tokio::test]
async fn practice_battle_result_decrements_group_exercise_quest_when_composition_matches() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 320;

	let ships = add_ships_with_type(&context, pid, KcShipType::DD, 4, 99).await;
	context
		.update_fleet_ships(pid, 1, &[ships[0], ships[1], ships[2], ships[3], -1, -1])
		.await
		.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;

	context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	let result = context.practice_battle_result(pid).await.unwrap();

	assert!(matches!(result.api_win_rank.as_str(), "S" | "A" | "B"));
	assert_eq!(exercise_times_remaining(&context, pid, quest_id).await, 3);
}

#[tokio::test]
async fn idle_exercise_quest_accumulates_progress_before_activation() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 301;

	let ships = add_ships_with_type(&context, pid, KcShipType::BB, 4, 99).await;
	context
		.update_fleet_ships(pid, 1, &[ships[0], ships[1], ships[2], ships[3], -1, -1])
		.await
		.unwrap();
	ensure_idle_quest(&context, pid, quest_id).await;

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;

	context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	context.practice_battle_result(pid).await.unwrap();

	assert_eq!(exercise_times_remaining(&context, pid, quest_id).await, 0);
	let after_result = raw_quest_record(&context, pid, quest_id).await;
	assert_eq!(after_result.status, quest::progress::Status::Idle);
	assert_eq!(after_result.progress, quest::progress::Progress::Eighty);

	context.quest_start(pid, quest_id).await.unwrap();
	let after_start = raw_quest_record(&context, pid, quest_id).await;
	assert_eq!(after_start.status, quest::progress::Status::Activated);
	assert_eq!(after_start.progress, quest::progress::Progress::Eighty);
	assert_eq!(exercise_times_remaining(&context, pid, quest_id).await, 1);
}

#[tokio::test]
async fn idle_exercise_quest_completes_after_activation_and_one_more_battle() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 303;

	let ships = add_ships_with_type(&context, pid, KcShipType::BB, 6, 99).await;
	context
		.update_fleet_ships(pid, 1, &[ships[0], ships[1], ships[2], ships[3], ships[4], ships[5]])
		.await
		.unwrap();
	ensure_idle_quest(&context, pid, quest_id).await;

	for _ in 0..3 {
		let rivals = context.get_practice_rivals(pid).await.unwrap();
		let enemy_id = rivals.rivals[0].id;
		context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
		context.practice_battle_result(pid).await.unwrap();
	}

	assert_eq!(quest_progress_of(&context, pid, quest_id).await, quest::progress::Progress::Eighty);
	assert_eq!(exercise_times_remaining(&context, pid, quest_id).await, 0);

	context.quest_start(pid, quest_id).await.unwrap();
	assert_eq!(exercise_times_remaining(&context, pid, quest_id).await, 1);

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;
	context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	context.practice_battle_result(pid).await.unwrap();

	assert_eq!(
		quest_progress_of(&context, pid, quest_id).await,
		quest::progress::Progress::Completed
	);
}

#[tokio::test]
async fn idle_group_exercise_quest_shows_initial_progress_after_matching_battle() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 320;

	let ships = add_ships_with_type(&context, pid, KcShipType::DD, 4, 99).await;
	context
		.update_fleet_ships(pid, 1, &[ships[0], ships[1], ships[2], ships[3], -1, -1])
		.await
		.unwrap();
	ensure_idle_quest(&context, pid, quest_id).await;

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;

	context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	context.practice_battle_result(pid).await.unwrap();

	assert_eq!(exercise_times_remaining(&context, pid, quest_id).await, 3);
	assert_eq!(quest_progress_of(&context, pid, quest_id).await, quest::progress::Progress::Half);
}
