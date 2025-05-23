use serde::{Deserialize, Serialize};

use crate::kc2::KcApiMission;

/// Expedition state
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Default)]
pub enum ExpeditionState {
	/// Not started
	#[default]
	NotStarted = 0,
	/// Unfinished
	Unfinished = 1,
	/// Completed
	Completed = 2,
}

/// User expedition record
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Expedition {
	/// Profile id
	pub id: i64,

	/// Expedition id
	pub mission_id: i64,

	/// Expedition state
	pub state: ExpeditionState,
}

impl From<Expedition> for KcApiMission {
	fn from(value: Expedition) -> Self {
		Self {
			api_mission_id: value.mission_id,
			api_state: value.state as i64,
		}
	}
}
