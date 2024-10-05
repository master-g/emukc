use async_trait::async_trait;
use emukc_db::{
	entity::profile::material,
	sea_orm::{entity::prelude::*, ActiveValue, TransactionTrait},
};
use emukc_model::kc2::MaterialCategory;

use crate::{err::GameplayError, prelude::HasContext};

/// A trait for material related gameplay.
#[async_trait]
pub trait MaterialOps {
	/// Add furniture to a profile.
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
}

#[async_trait]
impl<T: HasContext + ?Sized> MaterialOps for T {
	async fn add_material(
		&self,
		profile_id: i64,
		category: MaterialCategory,
		amount: i64,
	) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		add_material(&tx, profile_id, category, amount).await?;

		Ok(())
	}
}

/// Add furniture to a profile.
///
/// # Parameters
///
/// - `c`: The database connection.
/// - `profile_id`: The profile ID.
/// - `material`: The material category.
/// - `amount`: The amount of the material.
#[allow(unused)]
pub async fn add_material<C>(
	c: &C,
	profile_id: i64,
	material: MaterialCategory,
	amount: i64,
) -> Result<material::ActiveModel, GameplayError>
where
	C: ConnectionTrait,
{
	let Some(model) =
		material::Entity::find().filter(material::Column::ProfileId.eq(profile_id)).one(c).await?
	else {
		return Err(GameplayError::ProfileNotFound(profile_id));
	};

	let mut am: material::ActiveModel = model.clone().into();
	am.profile_id = ActiveValue::Unchanged(model.profile_id);

	match material {
		MaterialCategory::Fuel => am.fuel = ActiveValue::Set(model.fuel + amount),
		MaterialCategory::Ammo => am.ammo = ActiveValue::Set(model.ammo + amount),
		MaterialCategory::Steel => am.steel = ActiveValue::Set(model.steel + amount),
		MaterialCategory::Bauxite => am.bauxite = ActiveValue::Set(model.bauxite + amount),
		MaterialCategory::Torch => am.torch = ActiveValue::Set(model.torch + amount),
		MaterialCategory::Bucket => am.bucket = ActiveValue::Set(model.bucket + amount),
		MaterialCategory::DevMat => am.devmat = ActiveValue::Set(model.devmat + amount),
		MaterialCategory::Screw => am.screw = ActiveValue::Set(model.screw + amount),
	};

	let model = am.save(c).await?;

	Ok(model)
}
