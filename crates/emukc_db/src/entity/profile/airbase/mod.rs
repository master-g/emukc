//! User Airbase
use sea_orm::entity::prelude::*;

pub mod base;
pub mod extend;
pub mod plane;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());
	// base
	{
		let stmt = schema.create_table_from_entity(base::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
