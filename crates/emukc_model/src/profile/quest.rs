use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Kc3rdQuestPeriod, Kc3rdQuestRequirement};

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct QuestOnceRecord {
	/// profile id
	pub id: i64,

	/// quest id
	pub quest_id: i64,

	/// complete time
	pub complete_time: DateTime<Utc>,
}

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

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum QuestStatus {
	/// not started
	#[default]
	NotStarted = 1,
	/// in progress
	InProgress = 2,
	/// completed
	Completed = 3,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum QuestProgressStatus {
	/// empty
	#[default]
	Empty = 0,
	/// 50% or more
	Half = 1,
	/// 80% or more
	Eighty = 2,
}

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
