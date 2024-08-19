//! Rival ship entity

use emukc_model::profile::practice::RivalShip;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "rival_ship")]
pub struct Model {
	/// ship instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

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
		from = "Column::ProfileId",
		to = "super::rival::Column::Id"
	)]
	Rival,
}

impl Related<super::rival::Entity> for Entity {
	fn to() -> RelationDef {
		Relation::Rival.def()
	}
}

impl ActiveModelBehavior for ActiveModel {}

impl From<RivalShip> for ActiveModel {
	fn from(t: RivalShip) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(t.id),
			mst_id: ActiveValue::Set(t.mst_id),
			level: ActiveValue::Set(t.level),
			star: ActiveValue::Set(t.star),
		}
	}
}

impl From<Model> for RivalShip {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			instance_id: value.id,
			mst_id: value.mst_id,
			level: value.level,
			star: value.star,
		}
	}
}
