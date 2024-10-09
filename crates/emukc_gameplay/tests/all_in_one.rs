use emukc_db::prelude::new_mem_db;
use emukc_db::sea_orm::DbConn;
use emukc_gameplay::prelude::*;
use emukc_model::codex::Codex;
use emukc_model::kc2::{
	KcApiIncentiveItem, KcApiIncentiveMode, KcApiIncentiveType, MaterialCategory,
};

async fn mock_context() -> (DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let codex = Codex::load("../../.data/codex").unwrap();
	(db, codex)
}

async fn new_game_session() -> ((DbConn, Codex), AccountInfo, StartGameInfo) {
	let context = mock_context().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	(context, account, session)
}

#[tokio::test]
async fn incentive() {
	let (context, _, session) = new_game_session().await;

	let pid = session.profile.id;

	let codex = &context.1;
	let ship_mst = codex.manifest.find_ship(181).unwrap();

	context
		.add_incentive(
			pid,
			&[KcApiIncentiveItem {
				api_mode: KcApiIncentiveMode::PreRegister as i64,
				api_type: KcApiIncentiveType::Ship as i64,
				api_mst_id: ship_mst.api_id,
				api_getmes: ship_mst.api_getmes.clone(),
				api_slotitem_level: None,
				amount: 1,
				alv: 0,
			}],
		)
		.await
		.unwrap();

	let incentive = context.confirm_incentives(session.profile.id).await.unwrap();

	println!("{:?}", incentive);

	assert!(!incentive.is_empty());
}

#[tokio::test]
async fn add_ship() {
	let (context, _, session) = new_game_session().await;
	let ship = context.add_ship(session.profile.id, 951).await.unwrap();

	assert_eq!(ship.api_id, 1);
}

#[tokio::test]
async fn material() {
	let (context, _, session) = new_game_session().await;

	let old = context.get_materials(session.profile.id).await.unwrap();

	context.add_material(session.profile.id, MaterialCategory::Fuel, 100).await.unwrap();

	let new = context.get_materials(session.profile.id).await.unwrap();

	assert_eq!(new.fuel, old.fuel + 100);
}
