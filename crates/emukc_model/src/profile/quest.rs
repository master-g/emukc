use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::thirdparty::{Kc3rdQuestPeriod, Kc3rdQuestRequirement};

/// One-time quest record
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct QuestOneshotRecord {
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
	/// not activated
	#[default]
	Idle = 0,
	/// in progress
	Activated = 1,
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

	/// completed
	Completed = 3,
}

/// Quest progress record
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct QuestProgress {
	/// profile id
	pub id: i64,

	/// quest id
	pub quest_id: i64,

	/// state
	pub state: QuestStatus,

	/// progress
	pub progress: QuestProgressStatus,

	/// period
	pub period: Kc3rdQuestPeriod,

	/// start time
	pub start_time: DateTime<Utc>,

	/// requirements left to complete
	pub requirements: Kc3rdQuestRequirement,
}
