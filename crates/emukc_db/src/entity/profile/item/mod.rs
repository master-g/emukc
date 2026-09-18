//! User Items, use item, pay item, and slot item
use crate::entity::create_table;

pub mod pay_item;
pub mod picturebook;
pub mod slot_item;
pub mod use_item;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    // pay_item
    create_table(db, pay_item::Entity).await?;
    // slot_item
    create_table(db, slot_item::Entity).await?;
    // picturebook
    create_table(db, picturebook::Entity).await?;
    // use_item
    create_table(db, use_item::Entity).await?;

    Ok(())
}
