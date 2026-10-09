//! User Airbase
use crate::entity::create_table;

pub mod base;
pub mod plane;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    // base
    create_table(db, base::Entity).await?;
    // plane
    create_table(db, plane::Entity).await?;
    // ponytail: `since` came after the table shipped. The statement succeeds
    // exactly once, on a database made before it, and that is also the one
    // moment its rows still hold the wire's `api_cond` (always 1) where the
    // inner value now lives, so they are reset to a fresh deployment's 40.
    // Still no migration table: this and `preset_deck.locked` are the only two
    // such columns. Write one when a change cannot be told from "add failed".
    {
        use sea_orm::ConnectionTrait;
        let added = db
            .execute_unprepared(
                "ALTER TABLE plane_info ADD COLUMN since timestamp_with_timezone_text NULL",
            )
            .await
            .is_ok();
        if added {
            db.execute_unprepared("UPDATE plane_info SET condition = 40").await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use sea_orm::{ActiveModelTrait, ActiveValue, ConnectionTrait, EntityTrait, IntoActiveModel};

    use super::*;

    /// A database made before `plane_info.since` existed gains the column, its
    /// squadrons start from a fresh deployment's condition, and a timestamp
    /// written to the added column reads back.
    #[tokio::test]
    async fn an_older_plane_info_table_gains_the_since_column() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE plane_info (slot_id INTEGER PRIMARY KEY AUTOINCREMENT, profile_id BIGINT NOT NULL, \
             area_id BIGINT NOT NULL, rid BIGINT NOT NULL, squadron_id BIGINT NOT NULL, state INTEGER NOT NULL, \
             condition BIGINT NOT NULL, count BIGINT NOT NULL, max_count BIGINT NOT NULL); \
             INSERT INTO plane_info (slot_id, profile_id, area_id, rid, squadron_id, state, condition, count, max_count) \
             VALUES (7, 1, 6, 1, 1, 1, 1, 18, 18);",
        )
        .await
        .unwrap();

        bootstrap(&db).await.unwrap();

        let row = plane::Entity::find_by_id(7).one(&db).await.unwrap().unwrap();
        assert_eq!(row.condition, 40);
        assert_eq!(row.since, None);

        let now = chrono::Utc::now();
        let mut am = row.into_active_model();
        am.condition = ActiveValue::Set(12);
        am.since = ActiveValue::Set(Some(now));
        am.update(&db).await.unwrap();

        // A second start must neither trip over the column nor reset anything.
        bootstrap(&db).await.unwrap();

        let row = plane::Entity::find_by_id(7).one(&db).await.unwrap().unwrap();
        assert_eq!(row.condition, 12);
        assert_eq!(row.since, Some(now));
    }
}
