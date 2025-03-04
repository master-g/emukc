use async_trait::async_trait;
use emukc_db::{
	entity::profile::material,
	sea_orm::{ActiveValue, TransactionTrait, TryIntoModel, entity::prelude::*},
};
use emukc_model::{codex::Codex, kc2::MaterialCategory, profile::material::Material};

use crate::{err::GameplayError, gameplay::HasContext};

use super::basic::find_profile;

/// A trait for material related gameplay.
#[async_trait]
pub trait MaterialOps {
	/// Add material to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `values`: The materials to add.
	async fn add_material(
		&self,
		profile_id: i64,
		values: &[(MaterialCategory, i64)],
	) -> Result<(), GameplayError>;

	/// Deduct materials from a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `values`: The materials to deduct.
	async fn deduct_material(
		&self,
		profile_id: i64,
		values: &[(MaterialCategory, i64)],
	) -> Result<Material, GameplayError>;

	/// Get materials of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_materials(&self, profile_id: i64) -> Result<Material, GameplayError>;

	/// Update materials of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn update_materials(&self, profile_id: i64) -> Result<Material, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> MaterialOps for T {
	async fn add_material(
		&self,
		profile_id: i64,
		values: &[(MaterialCategory, i64)],
	) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		add_material_impl(&tx, codex, profile_id, values).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn deduct_material(
		&self,
		profile_id: i64,
		values: &[(MaterialCategory, i64)],
	) -> Result<Material, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let m = deduct_material_impl(&tx, profile_id, values).await?;

		tx.commit().await?;

		Ok(m.into())
	}

	async fn get_materials(&self, profile_id: i64) -> Result<Material, GameplayError> {
		let db = self.db();
		let record = get_mat_impl(db, profile_id).await?;
		let model: Material = record.into();

		Ok(model)
	}

	async fn update_materials(&self, profile_id: i64) -> Result<Material, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		let profile = find_profile(&tx, profile_id).await?;

		let m = update_materials_impl(&tx, codex, profile_id, profile.hq_level).await?;

		tx.commit().await?;

		Ok(m.into())
	}
}

pub(crate) async fn get_mat_impl<C>(
	c: &C,
	profile_id: i64,
) -> Result<material::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = material::Entity::find()
		.filter(material::Column::ProfileId.eq(profile_id))
		.one(c)
		.await?
		.ok_or(GameplayError::ProfileNotFound(profile_id))?;

	Ok(record)
}

/// Add material to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `values`: The materials to add.
#[allow(unused)]
pub(crate) async fn add_material_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	values: &[(MaterialCategory, i64)],
) -> Result<material::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = get_mat_impl(c, profile_id).await?;
	let mut model: Material = record.into();

	for (category, amount) in values.iter() {
		match category {
			MaterialCategory::Fuel => model.fuel += amount,
			MaterialCategory::Ammo => model.ammo += amount,
			MaterialCategory::Steel => model.steel += amount,
			MaterialCategory::Bauxite => model.bauxite += amount,
			MaterialCategory::Torch => model.torch += amount,
			MaterialCategory::Bucket => model.bucket += amount,
			MaterialCategory::DevMat => model.devmat += amount,
			MaterialCategory::Screw => model.screw += amount,
		};
	}

	let cfg = &codex.game_cfg.material;
	cfg.apply_hard_cap(&mut model);

	let am: material::ActiveModel = model.into();

	let am = am.save(c).await?;

	Ok(am.try_into_model()?)
}

pub(crate) async fn deduct_material_impl<C>(
	c: &C,
	profile_id: i64,
	values: &[(MaterialCategory, i64)],
) -> Result<material::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = get_mat_impl(c, profile_id).await?;
	let mut model: Material = record.into();

	for (category, amount) in values.iter() {
		if *amount <= 0 {
			continue;
		}
		let mut stocks = [
			(MaterialCategory::Fuel, &mut model.fuel),
			(MaterialCategory::Ammo, &mut model.ammo),
			(MaterialCategory::Steel, &mut model.steel),
			(MaterialCategory::Bauxite, &mut model.bauxite),
			(MaterialCategory::Torch, &mut model.torch),
			(MaterialCategory::Bucket, &mut model.bucket),
			(MaterialCategory::DevMat, &mut model.devmat),
			(MaterialCategory::Screw, &mut model.screw),
		];

		if let Some(stock) = stocks.iter_mut().find(|(c, _)| c == category) {
			if *stock.1 < *amount {
				return Err(GameplayError::Insufficient(format!(
					"material category:{:?}, has:{}, deduct:{}",
					category, *stock.1, amount
				)));
			}
			*stock.1 -= amount;
		}
	}

	let am: material::ActiveModel = model.into();

	let am = am.update(c).await?;

	Ok(am.try_into_model()?)
}

pub(crate) async fn update_materials_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	user_lv: i64,
) -> Result<material::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = get_mat_impl(c, profile_id).await?;
	let mut model: Material = record.into();
	codex.game_cfg.material.apply_self_replenish(&mut model, user_lv);

	let am = material::ActiveModel {
		profile_id: ActiveValue::Unchanged(profile_id),
		..model.into()
	};

	let am = am.save(c).await?;

	Ok(am.try_into_model()?)
}

/// Initialize material for a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `codex`: The codex.
/// - `profile_id`: The profile ID.
pub(super) async fn init<C>(c: &C, codex: &Codex, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let cfg = &codex.game_cfg.material;
	let model = cfg.new_material(profile_id);
	let am: material::ActiveModel = model.into();
	am.insert(c).await?;

	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	material::Entity::delete_by_id(profile_id).exec(c).await?;

	Ok(())
}
