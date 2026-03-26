//! Practice battle integration tests.

use emukc_db::prelude::new_mem_db;
use emukc_gameplay::prelude::*;
use emukc_model::codex::Codex;

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

#[tokio::test]
async fn practice_battle_and_result_flow_updates_rival_status() {
	let (context, session) = new_game_session().await;
	let pid = session.profile.id;

	let ship = context.add_ship(pid, 951).await.unwrap();
	context.update_fleet_ships(pid, 1, &[ship.api_id, -1, -1, -1, -1, -1]).await.unwrap();

	let rivals = context.get_practice_rivals(pid).await.unwrap();
	let enemy_id = rivals.rivals[0].id;

	let battle = context.practice_battle(pid, 1, 1, enemy_id).await.unwrap();
	assert_eq!(battle.api_deck_id, 1);
	assert_eq!(battle.api_formation, [1, 1, 1]);
	assert!(!battle.api_ship_ke.is_empty());

	let result = context.practice_battle_result(pid).await.unwrap();
	assert!(!result.api_win_rank.is_empty());
	assert_eq!(result.api_enemy_info.api_level, rivals.rivals[0].level);

	let rival = context.get_practice_rival_details(pid, enemy_id).await.unwrap();
	assert_ne!(rival.status as i64, 0);
}
