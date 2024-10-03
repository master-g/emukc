use emukc_db::prelude::new_mem_db;
use emukc_db::sea_orm::DbConn;
use emukc_gameplay::prelude::*;
use emukc_model::codex::Codex;

async fn mock_context() -> (DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let codex = Codex::default();
	(db, codex)
}

#[tokio::test]
async fn profile() {
	let context = mock_context().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();
	let session =
		context.start_game(&account.access_token.token, profile.profile.id).await.unwrap();

	let incentive = context.confirm_incentives(session.profile.id).await.unwrap();
	assert!(incentive.is_empty());
	println!("{:?}", session);
}
