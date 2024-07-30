use sea_orm::ConnectionTrait;

/// Account entity
pub mod account;
/// Token
pub mod token;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());

	// account
	{
		let stmt = schema.create_table_from_entity(account::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// token
	{
		let stmt = schema.create_table_from_entity(token::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
