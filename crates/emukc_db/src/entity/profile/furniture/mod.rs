//! Furniture entity module.
use crate::entity::create_table;

pub mod config;
pub mod record;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    // record
    create_table(db, record::Entity).await?;
    // config
    create_table(db, config::Entity).await?;

    Ok(())
}
