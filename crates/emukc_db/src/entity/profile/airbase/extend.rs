//! Airbase extend info

use emukc_model::profile::airbase::AirbaseExtendedInfo;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "airbase_extend")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Area ID
	pub area_id: i64,

	/// Maintenance level
	pub maintenance_level: i64,
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

impl From<AirbaseExtendedInfo> for ActiveModel {
	fn from(value: AirbaseExtendedInfo) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(value.id),
			area_id: ActiveValue::Set(value.area_id),
			maintenance_level: ActiveValue::Set(value.maintenance_level),
		}
	}
}

impl From<Model> for AirbaseExtendedInfo {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			area_id: value.area_id,
			maintenance_level: value.maintenance_level,
		}
	}
}
