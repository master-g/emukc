//! Periodic quest record

use chrono::{DateTime, Utc};
use emukc_model::{profile::quest::QuestPeriodicRecord, thirdparty::Kc3rdQuestPeriod};
use sea_orm::{entity::prelude::*, ActiveValue};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Period {
	/// Oneshot
	#[sea_orm(num_value = 1)]
	Oneshot,

	/// Daily
	#[sea_orm(num_value = 2)]
	Daily,

	/// Weekly
	#[sea_orm(num_value = 3)]
	Weekly,

	/// Daily3rd7th0th
	#[sea_orm(num_value = 4)]
	Daily3rd7th0th,

	/// Daily2nd8th
	#[sea_orm(num_value = 5)]
	Daily2nd8th,

	/// Monthly
	#[sea_orm(num_value = 6)]
	Monthly,

	/// Quarterly
	#[sea_orm(num_value = 7)]
	Quarterly,

	/// Annually
	#[sea_orm(num_value = 8)]
	Annually,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "quest_record_periodic")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Quest ID
	pub quest_id: i64,

	/// Complete time
	pub complete_time: DateTime<Utc>,

	/// Period
	pub period: Period,
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

impl From<Period> for Kc3rdQuestPeriod {
	fn from(value: Period) -> Self {
		match value {
			Period::Oneshot => Kc3rdQuestPeriod::Oneshot,
			Period::Daily => Kc3rdQuestPeriod::Daily,
			Period::Weekly => Kc3rdQuestPeriod::Weekly,
			Period::Daily3rd7th0th => Kc3rdQuestPeriod::Daily3rd7th0th,
			Period::Daily2nd8th => Kc3rdQuestPeriod::Daily2nd8th,
			Period::Monthly => Kc3rdQuestPeriod::Monthly,
			Period::Quarterly => Kc3rdQuestPeriod::Quarterly,
			Period::Annually => Kc3rdQuestPeriod::Annual,
		}
	}
}

impl From<Kc3rdQuestPeriod> for Period {
	fn from(value: Kc3rdQuestPeriod) -> Self {
		match value {
			Kc3rdQuestPeriod::Oneshot => Period::Oneshot,
			Kc3rdQuestPeriod::Daily => Period::Daily,
			Kc3rdQuestPeriod::Weekly => Period::Weekly,
			Kc3rdQuestPeriod::Daily3rd7th0th => Period::Daily3rd7th0th,
			Kc3rdQuestPeriod::Daily2nd8th => Period::Daily2nd8th,
			Kc3rdQuestPeriod::Monthly => Period::Monthly,
			Kc3rdQuestPeriod::Quarterly => Period::Quarterly,
			Kc3rdQuestPeriod::Annual => Period::Annually,
		}
	}
}

impl From<QuestPeriodicRecord> for ActiveModel {
	fn from(record: QuestPeriodicRecord) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(record.id),
			quest_id: ActiveValue::Set(record.quest_id),
			complete_time: ActiveValue::Set(record.complete_time),
			period: ActiveValue::Set(record.period.into()),
		}
	}
}

impl From<Model> for QuestPeriodicRecord {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			quest_id: value.quest_id,
			complete_time: value.complete_time,
			period: value.period.into(),
		}
	}
}
