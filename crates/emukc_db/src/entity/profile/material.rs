//! Material Entity

use chrono::{DateTime, Utc};
use emukc_model::profile::material::Material;
use sea_orm::{ActiveValue, entity::prelude::*};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "material")]
pub struct Model {
	/// Primary key
	#[sea_orm(primary_key)]
	pub profile_id: i64,

	/// Fuel
	pub fuel: i64,

	/// Ammo
	pub ammo: i64,

	/// Steel
	pub steel: i64,

	/// Bauxite
	pub bauxite: i64,

	/// Torch
	pub torch: i64,

	/// Bucket
	pub bucket: i64,

	/// Development material
	pub devmat: i64,

	/// Screw
	pub screw: i64,

	/// last time update first three materials
	pub last_update_primary: DateTime<Utc>,

	/// last time update bauxite
	pub last_update_bauxite: DateTime<Utc>,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(belongs_to = "super::Entity", from = "Column::ProfileId", to = "super::Column::Id")]
	Profile,
}

impl Related<super::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Material> for ActiveModel {
	fn from(t: Material) -> Self {
		Self {
			profile_id: ActiveValue::Set(t.id),
			fuel: ActiveValue::Set(t.fuel),
			ammo: ActiveValue::Set(t.ammo),
			steel: ActiveValue::Set(t.steel),
			bauxite: ActiveValue::Set(t.bauxite),
			torch: ActiveValue::Set(t.torch),
			bucket: ActiveValue::Set(t.bucket),
			devmat: ActiveValue::Set(t.devmat),
			screw: ActiveValue::Set(t.screw),
			last_update_primary: ActiveValue::Set(t.last_update_primary),
			last_update_bauxite: ActiveValue::Set(t.last_update_bauxite),
		}
	}
}

impl From<Model> for Material {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			fuel: value.fuel,
			ammo: value.ammo,
			steel: value.steel,
			bauxite: value.bauxite,
			torch: value.torch,
			bucket: value.bucket,
			devmat: value.devmat,
			screw: value.screw,
			last_update_primary: value.last_update_primary,
			last_update_bauxite: value.last_update_bauxite,
		}
	}
}
