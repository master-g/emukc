//! Sortie battle integration tests.

use std::path::PathBuf;

use emukc_db::{
	entity::profile::{self, map_record, ship},
	prelude::new_mem_db,
	sea_orm::{
		ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
	},
};
use emukc_gameplay::prelude::*;
use emukc_model::codex::Codex;
use emukc_time::chrono::{TimeZone, Utc};

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
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, session)
}

async fn new_game_session_with_maps() -> ((emukc_db::sea_orm::DbConn, Codex), StartGameInfo) {
	let context = mock_context_with_maps().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, session)
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
