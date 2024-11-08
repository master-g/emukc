//! User oss settings Entity

#![allow(missing_docs)]

use emukc_model::kc2::KcApiOssSetting;
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
#[sea_orm(table_name = "oss_settings")]
pub struct Model {
	/// Primary key
	#[sea_orm(primary_key)]
	pub profile_id: i64,

	/// language type
	pub language: Language,
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

impl From<KcApiOssSetting> for ActiveModel {
	fn from(value: KcApiOssSetting) -> Self {
		Self {
			profile_id: ActiveValue::NotSet,
			language: ActiveValue::Set(Language::n(value.api_language_type).unwrap()),
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

impl From<Model> for KcApiOssSetting {
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
		}
	}
}
