//! User Items, use item, pay item, and slot item
use sea_orm::entity::prelude::*;

pub mod pay_item;
pub mod picturebook;
pub mod slot_item;
pub mod use_item;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());
	// pay_item
	{
		let stmt = schema.create_table_from_entity(pay_item::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// slot_item
	{
		let stmt = schema.create_table_from_entity(slot_item::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// picturebook
	{
		let stmt = schema.create_table_from_entity(picturebook::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	// use_item
	{
		let stmt = schema.create_table_from_entity(use_item::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
