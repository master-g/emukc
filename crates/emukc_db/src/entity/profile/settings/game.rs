//! User game settings Entity

#![allow(missing_docs)]

use emukc_model::kc2::KcApiGameSetting;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "game_settings")]
pub struct Model {
	/// Primary key
	#[sea_orm(primary_key)]
	pub profile_id: i64,

	/// Secretary ship position id
	pub position_id: i64,

	/// port bgm id
	pub port_bgm: i64,

	/// friend fleet request flag
	pub friend_fleet_req_flag: bool,

	/// friend fleet request type
	pub friend_fleet_req_type: i64,
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

impl From<KcApiGameSetting> for ActiveModel {
	fn from(value: KcApiGameSetting) -> Self {
		Self {
			profile_id: ActiveValue::NotSet,
			position_id: ActiveValue::Set(value.api_position_id),
			port_bgm: ActiveValue::Set(value.api_p_bgm_id),
			friend_fleet_req_flag: ActiveValue::Set(value.api_friend_fleet_request_flag.eq(&1)),
			friend_fleet_req_type: ActiveValue::Set(value.api_friend_fleet_request_type),
		}
	}
}

impl From<Model> for KcApiGameSetting {
	fn from(value: Model) -> Self {
		Self {
			api_position_id: value.position_id,
			api_p_bgm_id: value.port_bgm,
			api_friend_fleet_request_flag: if value.friend_fleet_req_flag {
				1
			} else {
				0
			},
			api_friend_fleet_request_type: value.friend_fleet_req_type,
		}
	}
}
