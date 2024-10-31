use chrono::{DateTime, Utc};
use emukc_time::KcTime;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::kc2::KcApiNDock;

/// Repair dock status
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum RepairDockStatus {
	/// Locked
	Locked = -1,
	/// Idle
	Idle = 0,
	/// In repair
	Busy = 1,
}

/// Repair context
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RepairContext {
	/// Ship ID
	pub ship_id: i64,

	/// complete time
	pub complete_time: DateTime<Utc>,

	/// fuel consumption
	pub fuel: i64,

	/// steel consumption
	pub steel: i64,

	/// last update time
	pub last_update: DateTime<Utc>,
}

/// Repair dock, `NDock`
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RepairDock {
	/// Profile ID
	pub id: i64,

	/// Dock ID
	pub index: i64,

	/// status
	pub status: RepairDockStatus,

	/// context
	pub context: Option<RepairContext>,
}

/// Repair dock error
#[derive(Error, Debug)]
pub enum RepairDockError {
	/// Dock ID out of range
	#[error("Dock ID out of range: {0}")]
	OutOfRange(i64),
}

impl RepairDock {
	/// Create a new repair dock
	///
	/// # Arguments
	///
	/// * `id` - Profile ID
	/// * `index` - Dock ID
	///
	pub fn new(id: i64, index: i64) -> Result<Self, RepairDockError> {
		if !(1..=4).contains(&index) {
			return Err(RepairDockError::OutOfRange(index));
		}

		Ok(Self {
			id,
			index,
			status: if index < 3 {
				RepairDockStatus::Idle
			} else {
				RepairDockStatus::Locked
			},
			context: None,
		})
	}
}

impl From<RepairDock> for KcApiNDock {
	fn from(value: RepairDock) -> Self {
		Self {
			api_member_id: value.id,
			api_id: value.index,
			api_state: value.status as i64,
			api_ship_id: value.context.as_ref().map_or(0, |c| c.ship_id),
			api_complete_time: value
				.context
				.as_ref()
				.map_or(0, |c| c.complete_time.timestamp_millis()),
			api_complete_time_str: value.context.as_ref().map_or("0".to_owned(), |c| {
				KcTime::format_date(c.complete_time.timestamp_millis(), " ")
			}),
			api_item1: value.context.as_ref().map_or(0, |c| c.fuel),
			api_item2: 0,
			api_item3: value.context.as_ref().map_or(0, |c| c.steel),
			api_item4: 0,
		}
	}
}
