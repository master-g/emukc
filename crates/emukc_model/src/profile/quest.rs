use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::thirdparty::{Kc3rdQuestPeriod, Kc3rdQuestRequirement};

/// One-time quest record
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct QuestOneTimeRecord {
	/// profile id
	pub id: i64,

	/// quest id
	pub quest_id: i64,

	/// complete time
	pub complete_time: DateTime<Utc>,
}

/// Periodic quest record
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct QuestPeriodicRecord {
	/// profile id
	pub id: i64,

	/// quest id
	pub quest_id: i64,

	/// complete time
	pub complete_time: DateTime<Utc>,

	/// period
	pub period: Kc3rdQuestPeriod,
}

/// Quest status
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub enum QuestStatus {
	/// not started
	#[default]
	NotStarted = 1,
	/// in progress
	InProgress = 2,
	/// completed
	Completed = 3,
}

/// Quest progress status
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub enum QuestProgressStatus {
	/// empty
	#[default]
	Empty = 0,
	/// 50% or more
	Half = 1,
	/// 80% or more
	Eighty = 2,
}

/// Quest progress record
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct QuestProgress {
	/// profile id
	pub id: i64,

	/// quest id
	pub quest_id: i64,

	/// activated
	pub activated: bool,

	/// state
	pub state: QuestStatus,

	/// progress
	pub progress: QuestProgressStatus,

	/// requirements left to complete
	pub requirements: Kc3rdQuestRequirement,
}
