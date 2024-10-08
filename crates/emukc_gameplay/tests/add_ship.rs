use emukc_db::prelude::new_mem_db;
use emukc_db::sea_orm::DbConn;
use emukc_gameplay::prelude::*;
use emukc_model::codex::Codex;

fn load_codex() -> anyhow::Result<Codex> {
	let codex = Codex::load("../../.data/codex")?;
	Ok(codex)
}

async fn mock_context() -> (DbConn, Codex) {
	let db = new_mem_db().await.unwrap();
	let codex = load_codex().unwrap();
	(db, codex)
}

#[test_log::test(tokio::test)]
async fn profile() {
	let context = mock_context().await;

	let account = context.sign_up("test", "1234567").await.unwrap();
	let profile = context.new_profile(&account.access_token.token, "admin").await.unwrap();

	let ship = context.add_ship(profile.profile.id, 951).await.unwrap();

	println!("{:?}", ship);
}
