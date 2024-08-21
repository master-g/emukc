//! Furniture inventory entity

use emukc_model::profile::furniture::Furniture;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "furniture_record")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Furniture ID
	pub furniture_id: i64,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::ProfileId",
		to = "crate::entity::profile::Column::Id"
	)]
	Profile,
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Furniture> for ActiveModel {
	fn from(furniture: Furniture) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(furniture.id),
			furniture_id: ActiveValue::set(furniture.furniture_id),
		}
	}
}

impl From<Model> for Furniture {
	fn from(model: Model) -> Self {
		Self {
			id: model.profile_id,
			furniture_id: model.furniture_id,
		}
	}
}
