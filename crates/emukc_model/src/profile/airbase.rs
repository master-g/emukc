use serde::{Deserialize, Serialize};

use crate::{KcApiAirBase, KcApiAirBaseExpandedInfo, KcApiDistance, KcApiPlaneInfo};

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum AirbaseAction {
	#[default]
	IDLE = 0,
	ATTACK = 1,
	DEFENSE = 2,
	EVASION = 3,
	RESORT = 4,
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
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub enum PlaneState {
	#[default]
	UNASSIGNED = 0,
	ASSIGNED = 1,
	REASSIGNING = 2,
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

	/// Slot id (index)
	pub slot_id: i64,

	/// Squadron id
	pub squadron_id: i64,

	/// plane status
	pub state: PlaneState,

	/// plane condition
	pub condition: Option<i64>,

	/// plane count
	pub count: Option<i64>,

	/// plane max count
	pub max_count: Option<i64>,
}

/// Air base extended info
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct AirbaseExtendedInfo {
	/// Profile id
	pub id: i64,

	/// Airbase area id
	pub area_id: i64,

	/// maintenance level
	pub maintenance_level: i64,
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

impl From<AirbaseExtendedInfo> for KcApiAirBaseExpandedInfo {
	fn from(value: AirbaseExtendedInfo) -> Self {
		Self {
			api_area_id: value.area_id,
			api_maintenance_level: value.maintenance_level,
		}
	}
}

impl From<PlaneInfo> for KcApiPlaneInfo {
	fn from(value: PlaneInfo) -> Self {
		Self {
			api_cond: value.condition,
			api_count: value.count,
			api_max_count: value.max_count,
			api_slotid: value.slot_id,
			api_squadron_id: value.squadron_id,
			api_state: value.state as i64,
		}
	}
}
