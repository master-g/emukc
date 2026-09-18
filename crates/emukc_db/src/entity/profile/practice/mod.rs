//! Practice entities
use crate::entity::create_table;

pub mod config;
pub mod detail;
pub mod rival;
pub mod rival_ship;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    // ship
    create_table(db, rival_ship::Entity).await?;
    // detail
    create_table(db, detail::Entity).await?;
    // config
    create_table(db, config::Entity).await?;
    // rival
    create_table(db, rival::Entity).await?;

    Ok(())
}
