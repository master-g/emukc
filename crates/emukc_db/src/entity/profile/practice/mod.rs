//! Practice entities
use sea_orm::entity::prelude::*;

pub mod config;
pub mod detail;
pub mod rival;
pub mod rival_ship;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());
	// ship
	{
		let stmt = schema.create_table_from_entity(rival_ship::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// detail
	{
		let stmt = schema.create_table_from_entity(detail::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// config
	{
		let stmt = schema.create_table_from_entity(config::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// rival
	{
		let stmt = schema.create_table_from_entity(rival::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
