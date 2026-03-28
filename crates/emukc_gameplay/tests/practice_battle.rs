//! Practice battle integration tests.

use emukc_db::{
	entity::profile::{self, ship},
	prelude::new_mem_db,
	sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, IntoActiveModel},
};
use emukc_gameplay::prelude::*;
use emukc_model::{codex::Codex, kc2::KcShipType};

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

fn first_ship_mst_by_type(codex: &Codex, ship_type: KcShipType) -> i64 {
	codex
		.manifest
		.api_mst_ship
		.iter()
		.find(|mst| KcShipType::n(mst.api_stype) == Some(ship_type))
		.map(|mst| mst.api_id)
		.unwrap()
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
