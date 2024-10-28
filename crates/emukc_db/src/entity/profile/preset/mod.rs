//! Preset entities
use sea_orm::entity::prelude::*;

pub mod preset_caps;
pub mod preset_deck;
pub mod preset_slot;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());
	// caps
	{
		let stmt = schema.create_table_from_entity(preset_caps::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// deck
	{
		let stmt = schema.create_table_from_entity(preset_deck::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// slot
	{
		let stmt = schema.create_table_from_entity(preset_slot::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
