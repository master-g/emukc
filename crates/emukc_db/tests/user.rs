#[cfg(test)]
mod test {
	use chrono::Utc;
	use emukc_db::entity::{self};
	use emukc_model::{
		profile::{Profile, user_item::UserItem},
		user::account::Account,
	};
	use sea_orm::{
		ActiveValue, ConnectionTrait, Database, DatabaseConnection, EntityTrait, Statement,
	};

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

	#[allow(unused)]
	async fn mem_db() -> DatabaseConnection {
		let db = Database::connect("sqlite::memory:").await.unwrap();
		entity::bootstrap(&db).await.unwrap();

		db
	}

	#[allow(unused)]
	async fn new_account(db: &DatabaseConnection, name: &str) -> Account {
		let account = entity::user::account::Entity::find()
			.from_raw_sql(Statement::from_sql_and_values(
				db.get_database_backend(),
				r#"SELECT * FROM "account" WHERE "name" = ?"#,
				[name.into()],
			))
			.one(db)
			.await
			.unwrap();

		if let Some(account) = account {
			account.into()
		} else {
			let mut new_account = Account {
				uid: 0,
				name: name.to_owned(),
				secret: "test secret".to_owned(),
				create_time: Utc::now(),
				last_login: Utc::now(),
			};

			let active_model = entity::user::account::ActiveModel::from(new_account.clone());
			let result =
				entity::user::account::Entity::insert(active_model).exec(db).await.unwrap();

			new_account.uid = result.last_insert_id;
			new_account
		}
	}

	#[allow(unused)]
	async fn new_profile(db: &DatabaseConnection, account: &Account, name: &str) -> Profile {
		let profile = entity::profile::Entity::find()
			.from_raw_sql(Statement::from_sql_and_values(
				db.get_database_backend(),
				r#"SELECT * FROM "profile" WHERE "account_id" = ? AND "name" = ?"#,
				[account.uid.into(), name.into()],
			))
			.one(db)
			.await
			.unwrap();

		if let Some(profile) = profile {
			profile.into()
		} else {
			let mut new_profile = Profile {
				id: 0,
				account_id: account.uid,
				world_id: 0,
				name: name.to_owned(),
			};

			let active_model = entity::profile::default_active_model(account.uid, name);
			let result = entity::profile::Entity::insert(active_model).exec(db).await.unwrap();

			new_profile.id = result.last_insert_id;
			new_profile
		}
	}

	#[allow(unused)]
	async fn new_use_item(db: &DatabaseConnection, profile: &Profile, mst_id: i64, count: i64) {
		let user_item = UserItem {
			id: profile.id,
			mst_id,
			count,
		};

		let active_model = entity::profile::item::use_item::ActiveModel::from(user_item.clone());
		let old_entry = entity::profile::item::use_item::Entity::find()
			.from_raw_sql(Statement::from_sql_and_values(
				db.get_database_backend(),
				r#"SELECT * FROM "use_item" WHERE "profile_id" = ? AND "mst_id" = ?"#,
				[profile.id.into(), mst_id.into()],
			))
			.one(db)
			.await
			.unwrap();
		if let Some(old_entry) = old_entry {
			entity::profile::item::use_item::Entity::update(
				entity::profile::item::use_item::ActiveModel {
					id: ActiveValue::Unchanged(old_entry.id),
					profile_id: ActiveValue::Unchanged(old_entry.profile_id),
					mst_id: ActiveValue::Unchanged(old_entry.mst_id),
					count: ActiveValue::Set(user_item.count + old_entry.count),
				},
			)
			.exec(db)
			.await
			.unwrap();
		} else {
			entity::profile::item::use_item::Entity::insert(active_model).exec(db).await.unwrap();
		}
	}

	#[tokio::test]
	async fn test_account() {
		let db = bootstrap_db().await;

		let account = new_account(&db, "test_account").await;
		let profile = new_profile(&db, &account, "test_profile").await;

		assert_eq!(account.name, "test_account");
		assert_eq!(profile.name, "test_profile");

		new_use_item(&db, &profile, 114, 514).await;
	}
}
