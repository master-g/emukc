//! Tests for `emukc_db`

#[cfg(test)]
mod test {
    use chrono::Utc;
    use emukc_db::entity::{self};
    use emukc_model::{
        profile::{Profile, user_item::UserItem},
        user::account::Account,
    };
    use sea_orm::{
        ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
        EntityTrait, QueryFilter, Statement,
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
        if db_path.exists() {
            std::fs::remove_file(&db_path).unwrap();
        }
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

    async fn legacy_map_record_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute(Statement::from_string(
            db.get_database_backend(),
            r#"
CREATE TABLE "map_record" (
	"id" integer NOT NULL PRIMARY KEY AUTOINCREMENT,
	"profile_id" integer NOT NULL,
	"map_id" integer NOT NULL,
	"cleared" integer NOT NULL,
	"last_cleared_at" text NULL,
	"last_reset_at" text NULL,
	"defeat_count" integer NULL,
	"current_hp" integer NULL,
	"gauge_index" integer NOT NULL,
	"variant_key" text NULL,
	"selected_rank" integer NOT NULL,
	"event_state" integer NULL
)
"#
            .to_string(),
        ))
        .await
        .unwrap();

        db
    }

    #[allow(unused)]
    async fn new_account(db: &DatabaseConnection, name: &str) -> Account {
        let account = entity::user::account::Entity::find()
            .filter(entity::user::account::Column::Name.eq(name))
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
            .filter(entity::profile::Column::AccountId.eq(account.uid))
            .filter(entity::profile::Column::Name.eq(name))
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
            .filter(entity::profile::item::use_item::Column::ProfileId.eq(profile.id))
            .filter(entity::profile::item::use_item::Column::MstId.eq(mst_id))
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

    #[tokio::test]
    async fn map_record_bootstrap_supports_stage_id_roundtrip() {
        let db = mem_db().await;
        let account = new_account(&db, "map-record-account").await;
        let profile = new_profile(&db, &account, "map-record-profile").await;
        entity::profile::map_record::ActiveModel {
            id: ActiveValue::NotSet,
            profile_id: ActiveValue::Set(profile.id),
            map_id: ActiveValue::Set(73),
            cleared: ActiveValue::Set(false),
            last_cleared_at: ActiveValue::Set(None),
            last_reset_at: ActiveValue::Set(None),
            defeat_count: ActiveValue::Set(Some(2)),
            current_hp: ActiveValue::Set(None),
            gauge_index: ActiveValue::Set(1),
            stage_id: ActiveValue::Set(Some("pre_p_unlock".to_string())),
            selected_rank: ActiveValue::Set(entity::profile::map_record::SelectedRank::NotSet),
            event_state: ActiveValue::Set(None),
            unlocked: ActiveValue::Set(true),
        }
        .insert(&db)
        .await
        .unwrap();

        let record = entity::profile::map_record::Entity::find().one(&db).await.unwrap().unwrap();
        assert_eq!(record.stage_id.as_deref(), Some("pre_p_unlock"));
    }

    #[tokio::test]
    async fn map_record_bootstrap_migrates_legacy_variant_key_to_stage_id() {
        let db = legacy_map_record_db().await;
        db.execute(Statement::from_string(
            db.get_database_backend(),
            r#"
INSERT INTO "map_record"
	("profile_id", "map_id", "cleared", "gauge_index", "variant_key", "selected_rank")
VALUES
	(1, 73, 0, 1, 'pre_p_unlock', 0)
"#
            .to_string(),
        ))
        .await
        .unwrap();

        entity::bootstrap(&db).await.unwrap();

        let record = entity::profile::map_record::Entity::find().one(&db).await.unwrap().unwrap();
        assert_eq!(record.stage_id.as_deref(), Some("pre_p_unlock"));
    }
}
