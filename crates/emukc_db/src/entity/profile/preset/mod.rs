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
    // slot
    create_table(db, preset_slot::Entity).await?;
    // dev_item
    create_table(db, preset_dev_item::Entity).await?;

    Ok(())
}
