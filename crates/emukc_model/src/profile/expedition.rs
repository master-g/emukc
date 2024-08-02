use serde::{Deserialize, Serialize};

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

// TODO: Implement build_api_item
