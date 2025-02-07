use serde::{Deserialize, Serialize};

use crate::kc2::{KcApiAirBase, KcApiDistance, KcApiPlaneInfo};

/// Airbase action assigned
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug, Default)]
pub enum AirbaseAction {
	/// Idle
	#[default]
	Idle = 0,
	/// Attack
	Attack = 1,
	/// Defense
	Defense = 2,
	/// Evasion
	Evasion = 3,
	/// Resort
	Resort = 4,
}

/// User airbase
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Airbase {
	/// Profile id
	pub id: i64,

	/// Airbase area id
	pub area_id: i64,

	/// Air base id
	pub rid: i64,

	/// Airbase action
	pub action: AirbaseAction,

	/// Airbase base range
	pub base_range: i64,

	/// Airbase range bonus
	pub bonus_range: i64,

	/// Airbase name
	pub name: String,

	/// maintenance level
	pub maintenance_level: i64,
}

/// Plane status
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum PlaneState {
	/// Unassigned
	#[default]
	Unassigned = 0,
	/// Assigned
	Assigned = 1,
	/// Reassigning
	Reassigning = 2,
}

/// User plane(air base) info
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct PlaneInfo {
	/// Profile id
	pub id: i64,

	/// Airbase area id
	pub area_id: i64,

	/// Airbase id
	pub rid: i64,

	/// Slot item instance id
	pub slot_id: i64,

	/// Squadron id, index, starts from 1, up to 4
	pub squadron_id: i64,

	/// plane status
	pub state: PlaneState,

	/// plane condition
	pub condition: i64,

	/// plane count
	pub count: i64,

	/// plane max count
	pub max_count: i64,
}

impl From<Airbase> for KcApiAirBase {
	fn from(value: Airbase) -> Self {
		Self {
			api_action_kind: value.action as i64,
			api_area_id: value.area_id,
			api_distance: KcApiDistance {
				api_base: value.base_range,
				api_bonus: value.bonus_range,
			},
			api_name: value.name.clone(),
			api_plane_info: vec![],
			api_rid: value.rid,
		}
	}
}

impl From<PlaneInfo> for KcApiPlaneInfo {
	fn from(value: PlaneInfo) -> Self {
		Self {
			api_cond: Some(value.condition),
			api_count: Some(value.count),
			api_max_count: Some(value.max_count),
			api_slotid: value.slot_id,
			api_squadron_id: value.squadron_id,
			api_state: value.state as i64,
		}
	}
}
