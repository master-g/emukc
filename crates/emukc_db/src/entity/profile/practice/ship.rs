//! Rival ship entity

use emukc_model::profile::practice::RivalShip;
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "rival_ship")]
pub struct Model {
	/// ship instance ID
	#[sea_orm(primary_key)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Rival id
	pub rival_id: i64,

	/// Ship manifest ID
	pub mst_id: i64,

	/// Ship level
	pub level: i64,

	/// Ship star, indicates the modernization level
	pub star: i64,
}

/// Relation
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
	/// Relation to `Rival`
	#[sea_orm(
		belongs_to = "super::rival::Entity",
		from = "Column::RivalId",
		to = "super::rival::Column::Id"
	)]
	Rival,

	/// Relation to `Profile`
	#[sea_orm(
		belongs_to = "crate::entity::profile::Entity",
		from = "Column::ProfileId",
		to = "crate::entity::profile::Column::Id"
	)]
	Profile,
}

impl Related<super::rival::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Rival.def()
	}
}

impl Related<crate::entity::profile::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Profile.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<Model> for RivalShip {
	fn from(value: Model) -> Self {
		Self {
			id: value.id,
			mst_id: value.mst_id,
			level: value.level,
			star: value.star,
		}
	}
}
