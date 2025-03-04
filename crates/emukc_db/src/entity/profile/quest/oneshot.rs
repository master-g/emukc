//! One-time quest record

use chrono::{DateTime, Utc};
use emukc_model::profile::quest::QuestOneshotRecord;
use sea_orm::{ActiveValue, entity::prelude::*};

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, DeriveEntityModel)]
#[sea_orm(table_name = "quest_record_oneshot")]
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

impl From<QuestOneshotRecord> for ActiveModel {
	fn from(record: QuestOneshotRecord) -> Self {
		Self {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(record.id),
			quest_id: ActiveValue::Set(record.quest_id),
			complete_time: ActiveValue::Set(record.complete_time),
		}
	}
}

impl From<Model> for QuestOneshotRecord {
	fn from(value: Model) -> Self {
		Self {
			id: value.profile_id,
			quest_id: value.quest_id,
			complete_time: value.complete_time,
		}
	}
}
