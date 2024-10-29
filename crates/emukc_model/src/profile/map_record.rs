#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::kc2::KcApiMapRecord;

/// User map record
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MapRecord {
	/// Map ID
	pub id: i64,

	/// Has cleared
	pub cleared: bool,

	/// How many airbase can be useds
	pub airbase_count: Option<i64>,

	/// Defeat context
	pub defeat_ctx: Option<MapDefeatContext>,

	/// Event context
	pub event_ctx: Option<MapEventContext>,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, enumn::N)]
pub enum MapGaugeType {
	/// Destroy gauge
	Destroy = 1,

	/// HP
	HP = 2,

	/// Landing (TP)
	Landing = 3,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, enumn::N)]
pub enum MapRefreshType {
	/// Never
	Never = 0,

	/// Monthly
	Monthly = 1,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct MapDefeatContext {
	/// current defeat count
	pub defeat_count: i64,

	/// defeat count required
	pub defeat_required: i64,

	/// gauge number
	pub gauge_num: i64,

	/// gauge type
	pub gauge_type: MapGaugeType,

	/// refresh type
	pub refresh_type: MapRefreshType,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, enumn::N)]
pub enum MapEventState {
	/// Default
	Default = 1,

	/// Completed
	Completed = 2,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct MapEventContext {
	/// Current HP
	pub now_hp: i64,

	/// Max HP
	pub max_hp: i64,

	/// Event state
	pub state: MapEventState,

	/// Flag
	/// [0, x, x] normal fleet enabled, [1, x, x] combined fleet enabled
	/// [x, 1, x] Carrier Task Force, [x, 2, x] Surface Task Force, [x, 4, x] Transport Escort Force
	/// [x, x, 0] ?, [x, x, 1] 7 ships enabled
	pub sally_flag: [i64; 3],

	// ?
	pub s_no: i64,

	// ?
	pub m10: i64,
}

/// List of map IDs
pub const MAP_ID_LIST: &[i64; 33] = &[
	11, 12, 13, 14, 15, // map 1
	21, 22, 23, 24, 25, // map 2
	31, 32, 33, 34, 35, // map 3
	41, 42, 43, 44, 45, // map 4
	51, 52, 53, 54, 55, // map 5
	61, 62, 63, 64, 65, // map 6
	71, 72, 73, // map 7
];

impl From<MapRecord> for KcApiMapRecord {
	fn from(value: MapRecord) -> Self {
		Self {
			api_id: value.id,
			api_cleared: value.cleared as i64,
			api_sally_flag: value.event_ctx.map(|x| x.sally_flag),
			api_defeat_count: value.defeat_ctx.map(|x| x.defeat_count),
			api_required_defeat_count: value.defeat_ctx.map(|x| x.defeat_required),
			api_gauge_type: value.defeat_ctx.map(|x| x.gauge_type as i64),
			api_gauge_num: value.defeat_ctx.map(|x| x.gauge_num),
			api_air_base_decks: value.airbase_count,
			api_s_no: value.event_ctx.map(|x| x.s_no),
			api_m10: value.event_ctx.map(|x| x.m10),
		}
	}
}

/// From map ID to map record
impl From<i64> for MapRecord {
	fn from(value: i64) -> Self {
		Self {
			id: value,
			cleared: false,
			airbase_count: None,
			defeat_ctx: None,
			event_ctx: None,
		}
	}
}

/// From map ID and airbase count to map record
impl From<(i64, i64)> for MapRecord {
	fn from(value: (i64, i64)) -> Self {
		Self {
			id: value.0,
			cleared: false,
			airbase_count: Some(value.1),
			defeat_ctx: None,
			event_ctx: None,
		}
	}
}

/// From map ID, airbase count and defeat context to map record
impl From<(i64, Option<i64>, MapDefeatContext)> for MapRecord {
	fn from(value: (i64, Option<i64>, MapDefeatContext)) -> Self {
		Self {
			id: value.0,
			cleared: false,
			airbase_count: value.1,
			defeat_ctx: Some(value.2),
			event_ctx: None,
		}
	}
}

pub static DEFAULT_MAP_RECORDS: LazyLock<Vec<MapRecord>> = LazyLock::new(|| {
	vec![
		11.into(),
		12.into(),
		13.into(),
		14.into(),
		(
			15,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 4,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Monthly,
			},
		)
			.into(),
		21.into(),
		22.into(),
		23.into(),
		24.into(),
		(
			25,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 4,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Never,
			},
		)
			.into(),
		31.into(),
		32.into(),
		33.into(),
		34.into(),
		(
			35,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 4,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Monthly,
			},
		)
			.into(),
		41.into(),
		42.into(),
		43.into(),
		(
			44,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 4,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Never,
			},
		)
			.into(),
		(
			45,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 5,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Monthly,
			},
		)
			.into(),
		51.into(),
		52.into(),
		53.into(),
		(
			54,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 5,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Never,
			},
		)
			.into(),
		(
			55,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 5,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Monthly,
			},
		)
			.into(),
		61.into(),
		(
			62,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 3,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Never,
			},
		)
			.into(),
		(
			63,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 4,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Never,
			},
		)
			.into(),
		(
			64,
			Some(1),
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 5,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Never,
			},
		)
			.into(),
		(
			65,
			Some(2),
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 6,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Monthly,
			},
		)
			.into(),
		(
			71,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 3,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Never,
			},
		)
			.into(),
		(
			72,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 3,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Monthly,
			},
		)
			.into(),
		(
			73,
			None,
			MapDefeatContext {
				defeat_count: 0,
				defeat_required: 3,
				gauge_num: 1,
				gauge_type: MapGaugeType::Destroy,
				refresh_type: MapRefreshType::Monthly,
			},
		)
			.into(),
	]
});
