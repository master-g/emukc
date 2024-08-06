use serde::{Deserialize, Serialize};

use crate::KcApiMission;

/// Expedition state
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ExpeditionState {
	/// Never started
	NeverStarted = 0,
	/// Unfinished
	Unfinished = 1,
	/// Completed
	Completed = 2,
}

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
