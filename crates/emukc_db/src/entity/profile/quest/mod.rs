//! Quest progress record
use sea_orm::entity::prelude::*;

pub mod oneshot;
pub mod periodic;
pub mod progress;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());
	// progress
	{
		let stmt = schema.create_table_from_entity(progress::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// oneshot
	{
		let stmt = schema.create_table_from_entity(oneshot::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// periodic
	{
		let stmt = schema.create_table_from_entity(periodic::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
