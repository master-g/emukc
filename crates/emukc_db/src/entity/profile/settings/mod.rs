//! Practice entities
use crate::entity::create_table;

pub mod game;
pub mod option;
pub mod oss;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    // game settings
    create_table(db, game::Entity).await?;
    // option settings
    create_table(db, option::Entity).await?;
    // oss
    create_table(db, oss::Entity).await?;

    Ok(())
}
