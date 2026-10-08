//! Preset entities
use crate::entity::create_table;

pub mod preset_caps;
pub mod preset_deck;
pub mod preset_dev_item;
pub mod preset_slot;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    // caps
    create_table(db, preset_caps::Entity).await?;
    // deck
    create_table(db, preset_deck::Entity).await?;
    // ponytail: the one column added after the table shipped, so a database made
    // before it gets it here; the statement fails harmlessly once it exists.
    // Write a migration step if a second such column ever comes along.
    {
        use sea_orm::ConnectionTrait;
        let _ = db
            .execute_unprepared(
                "ALTER TABLE preset_deck ADD COLUMN locked BOOLEAN NOT NULL DEFAULT FALSE",
            )
            .await;
    }
    // slot
    create_table(db, preset_slot::Entity).await?;
    // dev_item
    create_table(db, preset_dev_item::Entity).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use sea_orm::{ConnectionTrait, EntityTrait};

    use super::*;

    /// A database made before `preset_deck.locked` existed gains the column,
    /// and its rows read back unlocked.
    #[tokio::test]
    async fn an_older_preset_deck_table_gains_the_lock_column() {
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared(
            "CREATE TABLE preset_deck (id INTEGER PRIMARY KEY AUTOINCREMENT, profile_id BIGINT NOT NULL, \
             \"index\" BIGINT NOT NULL, name TEXT NOT NULL, ship_1 BIGINT NOT NULL, ship_2 BIGINT NOT NULL, \
             ship_3 BIGINT NOT NULL, ship_4 BIGINT NOT NULL, ship_5 BIGINT NOT NULL, ship_6 BIGINT NOT NULL); \
             INSERT INTO preset_deck (profile_id, \"index\", name, ship_1, ship_2, ship_3, ship_4, ship_5, ship_6) \
             VALUES (1, 1, 'old', 1, -1, -1, -1, -1, -1);",
        )
        .await
        .unwrap();

        bootstrap(&db).await.unwrap();
        // A second start must not trip over the column it added.
        bootstrap(&db).await.unwrap();

        let rows = preset_deck::Entity::find().all(&db).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].locked);
    }
}
