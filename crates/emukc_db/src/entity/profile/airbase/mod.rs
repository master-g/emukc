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

    Ok(())
}
