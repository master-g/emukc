//! Sortie battle integration tests.

use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};

use emukc_db::{
	entity::profile::{self, map_record, quest, ship},
	prelude::new_mem_db,
	sea_orm::{
		ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
	},
};
use emukc_gameplay::prelude::*;
use emukc_model::{codex::Codex, thirdparty::Kc3rdQuestRequirement};
use emukc_time::chrono::{TimeZone, Utc};

static PROFILE_ID_BUMP: AtomicI64 = AtomicI64::new(0);

async fn mock_context() -> (emukc_db::sea_orm::DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
	(db, codex)
}

async fn mock_context_with_maps() -> (emukc_db::sea_orm::DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let mut codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
	let kcdata_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.data/temp/kc_data");
	codex.load_maps_from_kcdata_root(kcdata_root).unwrap();
	(db, codex)
}

async fn new_game_session() -> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
	let context = mock_context().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let extra_profiles = PROFILE_ID_BUMP.fetch_add(1, Ordering::Relaxed);
	for idx in 0..extra_profiles {
		let name = format!("warmup-{extra_profiles}-{idx}");
		context.new_profile(&account.access_token.token, &name).await.unwrap();
	}
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, session)
}

async fn new_game_session_with_maps() -> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
	let context = mock_context_with_maps().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let extra_profiles = PROFILE_ID_BUMP.fetch_add(1, Ordering::Relaxed);
	for idx in 0..extra_profiles {
		let name = format!("warmup-maps-{extra_profiles}-{idx}");
		context.new_profile(&account.access_token.token, &name).await.unwrap();
	}
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, session)
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

fn path_to_boss(codex: &Codex, map_id: i64) -> Vec<i64> {
	let definition = codex.maps.map_definition(map_id).unwrap();
	let variant = definition.variant("").unwrap();
	let start = variant.first_progress_cell_no().unwrap();
	let boss = variant.boss_cell_no;

	fn dfs(
		variant: &emukc_model::codex::map::MapVariantDefinition,
		current: i64,
		target: i64,
		path: &mut Vec<i64>,
	) -> bool {
		path.push(current);
		if current == target {
			return true;
		}

		let Some(cell) = variant.cell(current) else {
			path.pop();
			return false;
		};
		for next in &cell.next_cells {
			if dfs(variant, *next, target, path) {
				return true;
			}
		}

		path.pop();
		false
	}

	let mut path = Vec::new();
	assert!(dfs(variant, start, boss, &mut path));
	path
}

#[tokio::test]
async fn sortie_start_battle_result_flow_updates_stats() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	let before_profile = profile::Entity::find_by_id(pid).one(&context.0).await.unwrap().unwrap();
	let before_ship = ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();

	let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	assert_eq!(start.api_maparea_id, 1);
	assert_eq!(start.api_mapinfo_no, 1);
	assert!(start.api_bosscell_no >= 1);

	let battle = context.sortie_battle(pid, 1).await.unwrap();
	assert_eq!(battle.api_deck_id, 1);
	assert!(!battle.api_ship_ke.is_empty());

	let result = context.sortie_battle_result(pid).await.unwrap();
	assert!(!result.api_win_rank.is_empty());
	assert_eq!(result.api_quest_name, "鎮守府正面海域");
	assert_eq!(result.api_quest_level, 1);

	let (profile, _) = context.get_user_basic(pid).await.unwrap();
	assert_eq!(profile.sortie_wins + profile.sortie_loses, 1);
	let after_profile = profile::Entity::find_by_id(pid).one(&context.0).await.unwrap().unwrap();
	let after_ship = ship::Entity::find_by_id(ship.api_id).one(&context.0).await.unwrap().unwrap();
	assert_eq!(after_profile.experience, before_profile.experience + result.api_get_exp);
	assert_eq!(after_ship.exp_now, before_ship.exp_now + result.api_get_ship_exp[1]);
}

#[tokio::test]
async fn loaded_map_catalog_supports_start_and_next_flow() {
	let (context, session) = new_game_session_with_maps().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	let start = context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	assert_eq!(start.api_maparea_id, 1);
	assert_eq!(start.api_mapinfo_no, 1);
	assert!(start.api_cell_data.len() >= 4);
	assert_eq!(start.api_no, 1);

	let next = context.next_sortie(pid, None).await.unwrap();
	assert_eq!(next.api_from_no, 1);
	assert_eq!(next.api_no, 2);
	context.sortie_goback_port(pid).await.unwrap();
}

#[tokio::test]
async fn sortie_airbattle_reuses_single_fleet_day_battle_flow() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	let battle = context.sortie_airbattle(pid, 1).await.unwrap();
	assert_eq!(battle.api_deck_id, 1);
	assert!(!battle.api_ship_ke.is_empty());
	context.sortie_goback_port(pid).await.unwrap();
}

#[tokio::test]
async fn sortie_goback_port_clears_pending_runtime_state() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	context.sortie_battle(pid, 1).await.unwrap();
	context.sortie_goback_port(pid).await.unwrap();

	assert!(context.sortie_battle_result(pid).await.is_err());
	assert!(context.next_sortie(pid, None).await.is_err());
}

#[tokio::test]
async fn sortie_battle_result_advances_generic_sortie_quest() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 202;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	context.sortie_battle(pid, 1).await.unwrap();
	context.sortie_battle_result(pid).await.unwrap();

	assert_eq!(
		quest_progress_of(&context, pid, quest_id).await,
		quest::progress::Progress::Completed
	);
}

#[tokio::test]
async fn sortie_battle_result_advances_boss_quest_on_real_boss_node() {
	let (context, session) = new_game_session_with_maps().await;
	let pid = session.profile.id;
	let quest_id = 204;
	let maparea_id = 1;
	let mapinfo_no = 2;
	let map_id = 12;

	let mut fleet_slots = [-1; 6];
	for slot in &mut fleet_slots {
		*slot = context.add_ship(pid, 951).await.unwrap().api_id;
	}
	context.update_fleet_ships(pid, 1, &fleet_slots).await.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	let start = context.start_sortie(pid, 1, maparea_id, mapinfo_no, 1).await.unwrap();
	let path = path_to_boss(&context.1, map_id);
	assert_eq!(start.api_no, path[0]);
	assert_eq!(start.api_bosscell_no, *path.last().unwrap());
	for next_cell in path.iter().skip(1) {
		let next = context.next_sortie(pid, Some(*next_cell)).await.unwrap();
		assert_eq!(next.api_no, *next_cell);
	}

	context.sortie_battle(pid, 1).await.unwrap();
	context.sortie_battle_result(pid).await.unwrap();
	assert_eq!(
		quest_progress_of(&context, pid, quest_id).await,
		quest::progress::Progress::Completed
	);
}

#[tokio::test]
async fn sortie_goback_port_does_not_advance_sortie_quest() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;
	let quest_id = 202;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	ensure_started_quest(&context, pid, quest_id).await;

	context.start_sortie(pid, 1, 1, 1, 1).await.unwrap();
	context.sortie_battle(pid, 1).await.unwrap();
	context.sortie_goback_port(pid).await.unwrap();

	assert_eq!(quest_progress_of(&context, pid, quest_id).await, quest::progress::Progress::Empty);
}

#[tokio::test]
async fn monthly_map_record_resets_on_map_info_read() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	context.get_map_infos(pid).await.unwrap();

	let record = map_record::Entity::find()
		.filter(map_record::Column::ProfileId.eq(pid))
		.filter(map_record::Column::MapId.eq(15))
		.one(&context.0)
		.await
		.unwrap()
		.unwrap();
	let mut am = record.into_active_model();
	am.cleared = ActiveValue::Set(true);
	am.defeat_count = ActiveValue::Set(Some(4));
	am.last_reset_at = ActiveValue::Set(Some(Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap()));
	am.update(&context.0).await.unwrap();

	let infos = context.get_map_infos(pid).await.unwrap();
	let map_15 = infos.into_iter().find(|info| info.api_id == 15).unwrap();
	assert_eq!(map_15.api_cleared, 0);
	assert_eq!(map_15.api_defeat_count, Some(0));
}

#[tokio::test]
async fn combined_type_is_persisted_after_validation() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship1 = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship1.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	assert!(context.set_combined_type(pid, 1).await.is_err());

	context.unlock_fleet(pid, 2).await.unwrap();
	let ship2 = context.add_ship(pid, 952).await.unwrap();
	context.update_fleet_ships(pid, 2, &[ship2.api_id, -1, -1, -1, -1, -1]).await.unwrap();
	assert_eq!(context.set_combined_type(pid, 1).await.unwrap(), 1);
	assert_eq!(context.get_combined_type(pid).await.unwrap(), 1);
}
