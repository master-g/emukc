#[cfg(test)]
mod test {
	use emukc_db::entity::{self, user};
	use sea_orm::{Database, DatabaseConnection};

	#[allow(unused)]
	fn temp_dir() -> std::path::PathBuf {
		let root =
			std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
		root.join(".data").join("temp")
	}

	#[allow(unused)]
	async fn bootstrap_db() -> DatabaseConnection {
		let db_path = temp_dir().join("emukc.db");
		let sqlite_url = format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap());
		println!("{:?}", sqlite_url);

		Database::connect(&sqlite_url).await.unwrap()
	}

	async fn mem_db() -> DatabaseConnection {
		let db = Database::connect("sqlite::memory:").await.unwrap();
		entity::bootstrap(&db).await.unwrap();

		db
	}

	#[tokio::test]
	async fn test_account() {
		let _db = mem_db().await;
	}
}
