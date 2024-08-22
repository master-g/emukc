//! User quest progress entity

use emukc_model::{
	profile::quest::{QuestProgress, QuestProgressStatus, QuestStatus},
	thirdparty::{Kc3rdQuestCondition, Kc3rdQuestRequirement},
};
use sea_orm::entity::prelude::*;

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Status {
	/// Not Started
	#[sea_orm(num_value = 1)]
	NotStarted,

	/// In Progress
	#[sea_orm(num_value = 2)]
	InProgress,

	/// Completed
	#[sea_orm(num_value = 3)]
	Completed,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum Progress {
	/// Empty
	#[sea_orm(num_value = 0)]
	Empty,

	/// 50% or more
	#[sea_orm(num_value = 1)]
	Half,

	/// 80% or more
	#[sea_orm(num_value = 2)]
	Eighty,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum RequirementType {
	/// All the requirements must be met
	#[sea_orm(num_value = 1)]
	And,

	/// One of the requirements must be met
	#[sea_orm(num_value = 2)]
	OneOf,

	/// Requirements must be met in sequence
	#[sea_orm(num_value = 3)]
	Sequential,
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "quest_progress")]
pub struct Model {
	/// Instance ID
	#[sea_orm(primary_key, auto_increment = true)]
	pub id: i64,

	/// Profile ID
	pub profile_id: i64,

	/// Quest ID
	pub quest_id: i64,

	/// Activated
	pub activated: bool,

	/// Status
	pub status: Status,

	/// Progress
	pub progress: Progress,

	/// Requirement type
	pub requirement_type: RequirementType,

	/// Requirements, which is too complex to be modeled, so it's stored as JSON
	pub requirements: serde_json::Value,
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

impl From<Status> for QuestStatus {
	fn from(value: Status) -> Self {
		match value {
			Status::NotStarted => QuestStatus::NotStarted,
			Status::InProgress => QuestStatus::InProgress,
			Status::Completed => QuestStatus::Completed,
		}
	}
}

impl From<QuestStatus> for Status {
	fn from(value: QuestStatus) -> Self {
		match value {
			QuestStatus::NotStarted => Status::NotStarted,
			QuestStatus::InProgress => Status::InProgress,
			QuestStatus::Completed => Status::Completed,
		}
	}
}

impl From<Progress> for QuestProgressStatus {
	fn from(value: Progress) -> Self {
		match value {
			Progress::Empty => QuestProgressStatus::Empty,
			Progress::Half => QuestProgressStatus::Half,
			Progress::Eighty => QuestProgressStatus::Eighty,
		}
	}
}

impl From<QuestProgressStatus> for Progress {
	fn from(value: QuestProgressStatus) -> Self {
		match value {
			QuestProgressStatus::Empty => Progress::Empty,
			QuestProgressStatus::Half => Progress::Half,
			QuestProgressStatus::Eighty => Progress::Eighty,
		}
	}
}

impl From<Model> for QuestProgress {
	fn from(value: Model) -> Self {
		let conditions: Vec<Kc3rdQuestCondition> =
			serde_json::from_value(value.requirements).unwrap();
		Self {
			id: value.profile_id,
			quest_id: value.quest_id,
			activated: value.activated,
			state: value.status.into(),
			progress: value.progress.into(),
			requirements: match value.requirement_type {
				RequirementType::And => Kc3rdQuestRequirement::And(conditions),
				RequirementType::OneOf => Kc3rdQuestRequirement::OneOf(conditions),
				RequirementType::Sequential => Kc3rdQuestRequirement::Sequential(conditions),
			},
		}
	}
}
