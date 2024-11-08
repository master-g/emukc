//! User option settings Entity

#![allow(missing_docs)]

use emukc_model::kc2::KcApiOptionSetting;
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "option_settings")]
pub struct Model {
	/// Primary key
	#[sea_orm(primary_key)]
	pub profile_id: i64,

	/// skin id
	pub skin_id: i64,

	/// bgm volume
	pub bgm_volume: i64,

	/// se volume
	pub se_volume: i64,

	/// voice volume
	pub voice_volume: i64,

	/// secretary idle voice enabled
	pub v_be_left: bool,

	/// mission completed voice enabled
	pub v_duty: bool,
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

impl From<KcApiOptionSetting> for ActiveModel {
	fn from(value: KcApiOptionSetting) -> Self {
		Self {
			profile_id: ActiveValue::NotSet,
			skin_id: ActiveValue::Set(value.api_skin_id),
			bgm_volume: ActiveValue::Set(value.api_vol_bgm),
			se_volume: ActiveValue::Set(value.api_vol_se),
			voice_volume: ActiveValue::Set(value.api_vol_voice),
			v_be_left: ActiveValue::Set(value.api_v_be_left.eq(&1)),
			v_duty: ActiveValue::Set(value.api_v_duty.eq(&1)),
		}
	}
}

impl From<Model> for KcApiOptionSetting {
	fn from(value: Model) -> Self {
		Self {
			api_skin_id: value.skin_id,
			api_vol_bgm: value.bgm_volume,
			api_vol_se: value.voice_volume,
			api_vol_voice: value.voice_volume,
			api_v_be_left: value.v_be_left as i64,
			api_v_duty: value.v_duty as i64,
		}
	}
}
