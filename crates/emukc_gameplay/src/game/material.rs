use async_trait::async_trait;
use emukc_db::{
	entity::profile::material,
	sea_orm::{entity::prelude::*, TransactionTrait, TryIntoModel},
};
use emukc_model::{codex::Codex, kc2::MaterialCategory, profile::material::Material};

use crate::{err::GameplayError, gameplay::HasContext};

/// A trait for material related gameplay.
#[async_trait]
pub trait MaterialOps {
	/// Add material to a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `category`: The material category.
	/// - `amount`: The amount of the material.
	async fn add_material(
		&self,
		profile_id: i64,
		category: MaterialCategory,
		amount: i64,
	) -> Result<(), GameplayError>;

	/// Get materials of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_materials(&self, profile_id: i64) -> Result<Material, GameplayError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> MaterialOps for T {
	async fn add_material(
		&self,
		profile_id: i64,
		category: MaterialCategory,
		amount: i64,
	) -> Result<(), GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let tx = db.begin().await?;

		add_material_impl(&tx, codex, profile_id, category, amount).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn get_materials(&self, profile_id: i64) -> Result<Material, GameplayError> {
		let db = self.db();
		let record = get_mat_impl(db, profile_id).await?;
		let model: Material = record.into();

		Ok(model)
	}
}

async fn get_mat_impl<C>(c: &C, profile_id: i64) -> Result<material::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = material::Entity::find()
		.filter(material::Column::ProfileId.eq(profile_id))
		.one(c)
		.await?
		.ok_or_else(|| GameplayError::ProfileNotFound(profile_id))?;

	Ok(record)
}

/// Add material to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `category`: The material category.
/// - `amount`: The amount of the material.
#[allow(unused)]
pub async fn add_material_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	category: MaterialCategory,
	amount: i64,
) -> Result<material::Model, GameplayError>
where
	C: ConnectionTrait,
{
	let record = get_mat_impl(c, profile_id).await?;
	let mut model: Material = record.into();

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

	let cfg = &codex.material_cfg;
	cfg.apply_hard_cap(&mut model);

	let am: material::ActiveModel = model.into();

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
pub(super) async fn init_material_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let cfg = &codex.material_cfg;
	let model = cfg.new_material(profile_id);
	let am: material::ActiveModel = model.into();
	am.insert(c).await?;

	Ok(())
}
