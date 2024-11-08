//! Practice entities
use sea_orm::entity::prelude::*;

pub mod game;
pub mod option;
pub mod oss;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());
	// game settings
	{
		let stmt = schema.create_table_from_entity(game::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// option settings
	{
		let stmt = schema.create_table_from_entity(option::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// oss
	{
		let stmt = schema.create_table_from_entity(oss::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
