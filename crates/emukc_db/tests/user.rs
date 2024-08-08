#[cfg(test)]
mod test {
	use emukc_db::entity::{
		self,
		global::id_generator::{self, IdType},
	};
	use sea_orm::{sea_query::OnConflict, ActiveValue, Database, DatabaseConnection, EntityTrait};

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

		let db = Database::connect(&sqlite_url).await.unwrap();
		entity::bootstrap(&db).await.unwrap();

		db
	}

	async fn mem_db() -> DatabaseConnection {
		let db = Database::connect("sqlite::memory:").await.unwrap();
		entity::bootstrap(&db).await.unwrap();

		db
	}

	async fn gen_id(db: &DatabaseConnection) -> i64 {
		let record = id_generator::Entity::find_by_id(IdType::Account).one(db).await.unwrap();
		let new_value = if let Some(record) = record {
			record.current + 1
		} else {
			1
		};
		id_generator::Entity::insert(id_generator::ActiveModel {
			id: ActiveValue::set(IdType::Account),
			current: ActiveValue::set(new_value),
		})
		.on_conflict(
			OnConflict::column(id_generator::Column::Id)
				.update_column(id_generator::Column::Current)
				.to_owned(),
		)
		.exec(db)
		.await
		.unwrap();

		new_value
	}

	#[tokio::test]
	async fn test_account() {
		let db = bootstrap_db().await;
		for _ in 0..10 {
			let id = gen_id(&db).await;
			println!("{:?}", id);
		}
	}
}
