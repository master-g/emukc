use sea_orm::ConnectionTrait;

/// Entity to keep various integer IDs
pub mod id_generator;

/// Bootstrap the database with the necessary tables
pub async fn bootstrap(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
	let schema = sea_orm::Schema::new(db.get_database_backend());

	// id_generator
	{
		let stmt = schema.create_table_from_entity(id_generator::Entity).if_not_exists().to_owned();
		db.execute(db.get_database_backend().build(&stmt)).await?;
	}

	Ok(())
}
