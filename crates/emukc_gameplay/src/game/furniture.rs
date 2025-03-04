use async_trait::async_trait;
use emukc_db::{
	entity::profile::{furniture, item::use_item},
	sea_orm::{ActiveValue, IntoActiveModel, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
	kc2::{KcApiFurniture, KcUseItemType},
	prelude::ApiMstFurniture,
	profile::furniture::FurnitureConfig,
};

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for furniture related gameplay.
#[async_trait]
pub trait FurnitureOps {
	/// Add furniture to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The furniture manifest ID.
	async fn add_furniture(&self, profile_id: i64, mst_id: i64) -> Result<(), GameplayError>;

	/// Buy furniture.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `mst_id`: The furniture manifest ID.
	/// - `price`: The price of the furniture.
	/// - `need_craftman`: Whether the furniture needs a craftman to build.
	async fn buy_furniture(
		&self,
		profile_id: i64,
		mst_id: i64,
		price: i64,
		need_craftman: bool,
	) -> Result<(), GameplayError>;

	/// Get furniture configuration.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_furniture_config(&self, profile_id: i64)
	-> Result<FurnitureConfig, GameplayError>;

	/// Update furniture configuration.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `config`: The new configuration.
	async fn update_furniture_config(
		&self,
		profile_id: i64,
		config: &FurnitureConfig,
	) -> Result<(), GameplayError>;

	/// Get furnitures of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_furnitures(&self, profile_id: i64) -> Result<Vec<KcApiFurniture>, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> FurnitureOps for T {
	async fn add_furniture(&self, profile_id: i64, mst_id: i64) -> Result<(), GameplayError> {
		let db = self.db();

		let tx = db.begin().await?;

		add_furniture_impl(&tx, profile_id, mst_id).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn buy_furniture(
		&self,
		profile_id: i64,
		mst_id: i64,
		price: i64,
		need_craftman: bool,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		buy_furniture_impl(&tx, profile_id, mst_id, price, need_craftman).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn get_furniture_config(
		&self,
		profile_id: i64,
	) -> Result<FurnitureConfig, GameplayError> {
		let db = self.db();
		let (_, cfg) = get_furniture_config_impl(db, profile_id).await?;

		Ok(cfg)
	}

	async fn update_furniture_config(
		&self,
		profile_id: i64,
		config: &FurnitureConfig,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_furniture_config_impl(&tx, profile_id, config).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn get_furnitures(&self, profile_id: i64) -> Result<Vec<KcApiFurniture>, GameplayError> {
		let codex = self.codex();
		let db = self.db();

		let models = get_furnitures_impl(db, profile_id).await?;

		let furnitures = models
			.iter()
			.filter_map(|m| {
				codex.find::<ApiMstFurniture>(&m.furniture_id).ok().map(|mst| KcApiFurniture {
					api_id: mst.api_id,
					api_furniture_type: mst.api_type,
					api_furniture_no: mst.api_no,
					api_furniture_id: mst.api_id,
				})
			})
			.collect();

		Ok(furnitures)
	}
}

/// Add furniture to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `mst_id`: The furniture master ID.
#[allow(unused)]
pub(crate) async fn add_furniture_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
) -> Result<furniture::record::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	if let Some(record) = furniture::record::Entity::find()
		.filter(furniture::record::Column::ProfileId.eq(profile_id))
		.filter(furniture::record::Column::FurnitureId.eq(mst_id))
		.one(c)
		.await?
	{
		return Ok(record.into());
	}

	let am = furniture::record::ActiveModel {
		id: ActiveValue::NotSet,
		profile_id: ActiveValue::Set(profile_id),
		furniture_id: ActiveValue::Set(mst_id),
	};

	let model = am.save(c).await?;

	Ok(model)
}

pub(crate) async fn buy_furniture_impl<C>(
	c: &C,
	profile_id: i64,
	mst_id: i64,
	price: i64,
	need_craftman: bool,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	if need_craftman {
		let Some(craftman) = use_item::Entity::find()
			.filter(use_item::Column::ProfileId.eq(profile_id))
			.filter(use_item::Column::MstId.eq(KcUseItemType::FurnitureCraftsman as i64))
			.one(c)
			.await?
		else {
			return Err(GameplayError::EntryNotFound("furniture craftman".to_string()));
		};

		if craftman.count < 1 {
			return Err(GameplayError::Insufficient("furniture craftman".to_string()));
		}

		let count = ActiveValue::Set(craftman.count - 1);

		let mut am = craftman.into_active_model();
		am.count = count;
		am.update(c).await?;
	}

	let Some(fcoins) = use_item::Entity::find()
		.filter(use_item::Column::ProfileId.eq(profile_id))
		.filter(use_item::Column::MstId.eq(KcUseItemType::FCoin as i64))
		.one(c)
		.await?
	else {
		return Err(GameplayError::EntryNotFound("furniture coin".to_string()));
	};

	if fcoins.count < price {
		return Err(GameplayError::Insufficient("furniture coin".to_string()));
	}

	let count = ActiveValue::Set(fcoins.count - price);
	let mut am = fcoins.into_active_model();
	am.count = count;
	am.update(c).await?;

	add_furniture_impl(c, profile_id, mst_id).await?;

	Ok(())
}

/// Get user furniture configuration.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
#[allow(unused)]
pub(crate) async fn get_furniture_config_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<(furniture::config::Model, FurnitureConfig), GameplayError>
where
	C: ConnectionTrait,
{
	let record = furniture::config::Entity::find()
		.filter(furniture::config::Column::Id.eq(profile_id))
		.one(c)
		.await?
		.ok_or(GameplayError::ProfileNotFound(profile_id))?;

	let cfg: FurnitureConfig = record.into();

	Ok((record, cfg))
}

/// Update furniture config.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `config`: The new configuration.
#[allow(unused)]
pub(crate) async fn update_furniture_config_impl<C>(
	c: &C,
	profile_id: i64,
	config: &FurnitureConfig,
) -> Result<furniture::config::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	let record = furniture::config::Entity::find()
		.filter(furniture::config::Column::Id.eq(profile_id))
		.one(c)
		.await?;

	let mut am = furniture::config::ActiveModel {
		id: record.map_or(ActiveValue::NotSet, |r| ActiveValue::Unchanged(r.id)),
		floor: ActiveValue::Set(config.floor),
		wallpaper: ActiveValue::Set(config.wallpaper),
		window: ActiveValue::Set(config.window),
		wall_hanging: ActiveValue::Set(config.wall_hanging),
		shelf: ActiveValue::Set(config.shelf),
		desk: ActiveValue::Set(config.desk),
		season: ActiveValue::Set(config.season),
	};

	let model = am.save(c).await?;

	Ok(model)
}

pub(crate) async fn get_furnitures_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<Vec<furniture::record::Model>, GameplayError>
where
	C: ConnectionTrait,
{
	let records = furniture::record::Entity::find()
		.filter(furniture::record::Column::ProfileId.eq(profile_id))
		.all(c)
		.await?;

	Ok(records)
}

pub(super) async fn init<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let cfg = FurnitureConfig::default();
	let ids = cfg.api_values();
	for id in ids {
		add_furniture_impl(c, profile_id, id).await?;
	}

	update_furniture_config_impl(c, profile_id, &cfg).await?;

	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	furniture::record::Entity::delete_many()
		.filter(furniture::record::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	let cfg = FurnitureConfig::default();
	update_furniture_config_impl(c, profile_id, &cfg).await?;

	Ok(())
}
