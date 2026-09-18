use crate::entity::create_table;

/// Account entity
pub mod account;
/// Token
pub mod token;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
    // account
    create_table(db, account::Entity).await?;
    // token
    create_table(db, token::Entity).await?;

    Ok(())
}
