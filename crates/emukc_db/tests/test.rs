#[cfg(test)]
mod test {
	use std::str::FromStr;

	use emukc_db::entity::kc2::api_mst::ApiManifestEntity;
	use emukc_db::entity::kc2::api_mst_ship;
	use emukc_db::sea_orm::entity::prelude::*;
	use emukc_model::start2::ApiManifest;
	use emukc_model::start2::ApiMstShip;
	use sea_orm::sea_query::OnConflict;
	use sea_orm::Database;

	#[tokio::test]
	async fn test_manifest() {
		let db = bootstrap_db().await;

		let mst = load_manifest();
		let models: Vec<api_mst_ship::ActiveModel> =
			mst.api_mst_ship.iter().map(|s| api_mst_ship::ActiveModel::from(s.clone())).collect();
		let chunks = models.chunks(100);
		for chunk in chunks {
			api_mst_ship::Entity::insert_many(chunk.to_vec())
				.on_conflict(OnConflict::column(api_mst_ship::Column::Id).do_nothing().to_owned())
				.exec(&db)
				.await
				.unwrap();
		}

		let ships = api_mst_ship::Entity::find().all(&db).await.unwrap();
		ships.iter().map(|m| ApiMstShip::from(m.clone())).for_each(|s| {
			let v = serde_json::to_string(&s);
			println!("{:?}", v);
		});
	}

	async fn bootstrap_db() -> DatabaseConnection {
		let db_path = temp_dir().join("emukc.db");
		let sqlite_url = format!("sqlite:{}?mode=rwc", db_path.to_str().unwrap());
		println!("{:?}", sqlite_url);
		let db = Database::connect(&sqlite_url).await.unwrap();

		ApiManifestEntity::create_table(&db).await.unwrap();

		db
	}

	fn temp_dir() -> std::path::PathBuf {
		let root =
			std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
		root.join(".data").join("temp")
	}

	fn load_manifest() -> ApiManifest {
		let json_path = temp_dir().join("start2.json");
		println!("{:?}", json_path);
		let raw = std::fs::read_to_string(json_path).unwrap();

		ApiManifest::from_str(&raw).unwrap()
	}
}
