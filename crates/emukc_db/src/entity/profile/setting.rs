//! User settings Entity

#![allow(missing_docs)]

use emukc_model::kc2::KcApiGameSetting;
use sea_orm::{entity::prelude::*, ActiveValue};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum, enumn::N)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Language {
	/// Japanese
	#[sea_orm(num_value = 0)]
	Japanese = 0,

	/// English
	#[sea_orm(num_value = 1)]
	English = 1,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "game_settings")]
pub struct Model {
	/// Primary key
	#[sea_orm(primary_key)]
	pub profile_id: i64,

	/// language type
	pub language: Language,

	/// Secretary ship position id
	pub position_id: i64,

	/// UI Skin ID
	pub skin_id: i64,

	/// port bgm id
	pub port_bgm: i64,

	/// friend fleet request flag
	pub friend_fleet_req_flag: bool,

	/// friend fleet request type
	pub friend_fleet_req_type: i64,

	/// Ship sorting filters
	pub oss_1: i64,
	/// Ship sorting filters
	pub oss_2: i64,
	/// Ship sorting filters
	pub oss_3: i64,
	/// Ship sorting filters
	pub oss_4: i64,
	/// Ship sorting filters
	pub oss_5: i64,
	/// Ship sorting filters
	pub oss_6: i64,
	/// Ship sorting filters
	pub oss_7: i64,
	/// Ship sorting filters
	pub oss_8: i64,
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

impl From<KcApiGameSetting> for ActiveModel {
	fn from(value: KcApiGameSetting) -> Self {
		Self {
			profile_id: ActiveValue::NotSet,
			language: ActiveValue::Set(Language::n(value.api_language_type).unwrap()),
			position_id: ActiveValue::Set(value.api_position_id),
			skin_id: ActiveValue::Set(value.api_skin_id),
			port_bgm: ActiveValue::Set(value.api_p_bgm_id),
			friend_fleet_req_flag: ActiveValue::Set(value.api_friend_fleet_request_flag.eq(&1)),
			friend_fleet_req_type: ActiveValue::Set(value.api_friend_fleet_request_type),
			oss_1: ActiveValue::Set(value.api_oss_items[0]),
			oss_2: ActiveValue::Set(value.api_oss_items[1]),
			oss_3: ActiveValue::Set(value.api_oss_items[2]),
			oss_4: ActiveValue::Set(value.api_oss_items[3]),
			oss_5: ActiveValue::Set(value.api_oss_items[4]),
			oss_6: ActiveValue::Set(value.api_oss_items[5]),
			oss_7: ActiveValue::Set(value.api_oss_items[6]),
			oss_8: ActiveValue::Set(value.api_oss_items[7]),
		}
	}
}

impl From<Model> for KcApiGameSetting {
	fn from(value: Model) -> Self {
		Self {
			api_language_type: value.language as i64,
			api_oss_items: [
				value.oss_1,
				value.oss_2,
				value.oss_3,
				value.oss_4,
				value.oss_5,
				value.oss_6,
				value.oss_7,
				value.oss_8,
			],
			api_position_id: value.position_id,
			api_skin_id: value.skin_id,
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
