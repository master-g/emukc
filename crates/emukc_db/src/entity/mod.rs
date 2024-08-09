use sea_orm::ConnectionTrait;

/// Entities for `EmuKC` global variables.
pub mod global;
/// Entities for `EmuKC` profile related stuff.
pub mod profile;
/// Entities for `EmuKC` user related stuff.
pub mod user;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());

	// global
	{
		let stmt = schema
			.create_table_from_entity(global::id_generator::Entity)
			.if_not_exists()
			.to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	// user
	{
		let stmt =
			schema.create_table_from_entity(user::account::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}
	{
		let stmt = schema.create_table_from_entity(user::token::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
