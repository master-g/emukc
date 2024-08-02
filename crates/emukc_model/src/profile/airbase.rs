use serde::{Deserialize, Serialize};

/// User airbase
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Airbase {
	/// Profile id
	pub id: i64,

	/// Airbase area id
	pub area_id: i64,

	/// Air base id
	pub rid: i64,

	/// Airbase base range
	pub base_range: i64,

	/// Airbase range bonus
	pub range_bonus: i64,

	/// Airbase name
	pub name: String,
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
	pub state: i64,

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
