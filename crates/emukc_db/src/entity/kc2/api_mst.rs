use std::ops::Deref;

use emukc_model::start2::ApiManifest;
use sea_orm::{sea_query::OnConflict, ConnectionTrait, DatabaseConnection, EntityTrait};

use super::{
	api_mst_bgm, api_mst_const, api_mst_equip_exslot, api_mst_equip_exslot_ship, api_mst_ship,
};

const CHUNK_SIZE: usize = 64;

/// Newtype for `ApiManifest`
pub struct ApiManifestEntity(pub ApiManifest);

impl Deref for ApiManifestEntity {
	type Target = ApiManifest;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl ApiManifestEntity {
	/// Create tables for all entities
	pub async fn create_table(db: &DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
		let schema = sea_orm::Schema::new(sea_orm::DatabaseBackend::Sqlite);

		// api_mst_bgm
		{
			let stmt =
				schema.create_table_from_entity(api_mst_bgm::Entity).if_not_exists().to_owned();
			db.execute(db.get_database_backend().build(&stmt)).await?;
		}
		// api_mst_const
		{
			let stmt =
				schema.create_table_from_entity(api_mst_const::Entity).if_not_exists().to_owned();
			db.execute(db.get_database_backend().build(&stmt)).await?;
		}
		// api_mst_equip_exslot
		{
			let stmt = schema
				.create_table_from_entity(api_mst_equip_exslot::Entity)
				.if_not_exists()
				.to_owned();
			db.execute(db.get_database_backend().build(&stmt)).await?;
		}
		// api_mst_equip_exslot_ship
		{
			let stmt = schema
				.create_table_from_entity(api_mst_equip_exslot_ship::Entity)
				.if_not_exists()
				.to_owned();
			db.execute(db.get_database_backend().build(&stmt)).await?;
		}
		// api_mst_ship
		{
			let stmt =
				schema.create_table_from_entity(api_mst_ship::Entity).if_not_exists().to_owned();
			db.execute(db.get_database_backend().build(&stmt)).await?;
		}

		Ok(())
	}

	/// Save the `ApiManifest` to the database
	pub async fn save(&self, db: &DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
		self.save_bgm(db).await?;
		self.save_const(db).await?;
		self.save_equip_exslot(db).await?;
		self.save_equip_exslot_ship(db).await?;
		self.save_ship(db).await?;

		Ok(())
	}

	async fn save_bgm(&self, db: &DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
		let models: Vec<api_mst_bgm::ActiveModel> =
			self.api_mst_bgm.iter().map(|s| api_mst_bgm::ActiveModel::from(s.clone())).collect();
		let chunks = models.chunks(CHUNK_SIZE);
		for chunk in chunks {
			api_mst_bgm::Entity::insert_many(chunk.to_vec())
				.on_conflict(OnConflict::column(api_mst_bgm::Column::Id).do_nothing().to_owned())
				.exec(db)
				.await?;
		}

		Ok(())
	}

	async fn save_const(&self, db: &DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
		let model: api_mst_const::ActiveModel = self.api_mst_const.clone().into();
		api_mst_const::Entity::insert(model)
			.on_conflict(OnConflict::column(api_mst_const::Column::Id).do_nothing().to_owned())
			.exec(db)
			.await?;

		Ok(())
	}

	async fn save_equip_exslot(
		&self,
		db: &DatabaseConnection,
	) -> Result<(), sea_orm::error::DbErr> {
		let exslot: &[i64] = &self.api_mst_equip_exslot;
		let model: api_mst_equip_exslot::ActiveModel = exslot.into();
		api_mst_equip_exslot::Entity::insert(model)
			.on_conflict(
				OnConflict::column(api_mst_equip_exslot::Column::Id).do_nothing().to_owned(),
			)
			.exec(db)
			.await?;

		Ok(())
	}

	async fn save_equip_exslot_ship(
		&self,
		db: &DatabaseConnection,
	) -> Result<(), sea_orm::error::DbErr> {
		let models: Vec<api_mst_equip_exslot_ship::ActiveModel> = self
			.api_mst_equip_exslot_ship
			.iter()
			.map(|(key, value)| {
				let id: i64 = key.parse().unwrap();
				(id, value.clone()).into()
			})
			.collect();
		let chunks = models.chunks(CHUNK_SIZE);
		for chunk in chunks {
			api_mst_equip_exslot_ship::Entity::insert_many(chunk.to_vec())
				.on_conflict(
					OnConflict::column(api_mst_equip_exslot_ship::Column::Id)
						.do_nothing()
						.to_owned(),
				)
				.exec(db)
				.await?;
		}

		Ok(())
	}

	async fn save_ship(&self, db: &DatabaseConnection) -> Result<(), sea_orm::error::DbErr> {
		let models: Vec<api_mst_ship::ActiveModel> =
			self.api_mst_ship.iter().map(|s| api_mst_ship::ActiveModel::from(s.clone())).collect();
		let chunks = models.chunks(CHUNK_SIZE);
		for chunk in chunks {
			api_mst_ship::Entity::insert_many(chunk.to_vec())
				.on_conflict(OnConflict::column(api_mst_ship::Column::Id).do_nothing().to_owned())
				.exec(db)
				.await?;
		}

		Ok(())
	}
}
