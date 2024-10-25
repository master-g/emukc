use chrono::{DateTime, Utc};
use emukc_time::format_date;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::kc2::KcApiKDock;

/// Construction dock status
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum ConstructionDockStatus {
	/// Locked
	Locked = -1,
	/// Idle
	Idle = 0,
	/// In construction
	Busy = 2,
	/// Construction completed
	Completed = 3,
}

/// Construction context
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConstructionContext {
	/// Ship ID
	pub ship_id: i64,

	/// complete time
	pub complete_time: DateTime<Utc>,

	/// is current constuction large
	pub is_large: bool,

	/// fuel consumption
	pub fuel: i64,

	/// ammo consumption
	pub ammo: i64,

	/// steel consumption
	pub steel: i64,

	/// bauxite consumption
	pub bauxite: i64,

	/// development material consumption
	pub devmat: i64,
}

/// Construction dock, `KDock`
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConstructionDock {
	/// Profile ID
	pub id: i64,

	/// Dock ID
	pub index: i64,

	/// status
	pub status: ConstructionDockStatus,

	/// construction context
	pub context: Option<ConstructionContext>,
}

/// Construction dock error
#[derive(Error, Debug)]
pub enum ConstructionDockError {
	/// Dock ID out of range
	#[error("Dock ID out of range: {0}")]
	OutOfRange(i64),
}

impl ConstructionDock {
	/// Create a new construction dock
	///
	/// # Arguments
	///
	/// * `id` - Profile ID
	/// * `index` - Dock ID
	///
	pub fn new(id: i64, index: i64) -> Result<Self, ConstructionDockError> {
		if !(1..=4).contains(&index) {
			return Err(ConstructionDockError::OutOfRange(index));
		}

		Ok(Self {
			id,
			index,
			status: if index < 3 {
				ConstructionDockStatus::Idle
			} else {
				ConstructionDockStatus::Locked
			},
			context: None,
		})
	}
}

impl From<ConstructionDock> for KcApiKDock {
	fn from(value: ConstructionDock) -> Self {
		Self {
			api_id: value.index,
			api_state: value.status as i64,
			api_created_ship_id: value.context.as_ref().map_or(0, |c| c.ship_id),
			api_complete_time: value
				.context
				.as_ref()
				.map_or(0, |c| c.complete_time.timestamp_millis()),
			api_complete_time_str: value
				.context
				.as_ref()
				.map_or("0".to_owned(), |c| format_date(c.complete_time.timestamp_millis(), " ")),
			api_item1: value.context.as_ref().map_or(0, |c| c.fuel),
			api_item2: value.context.as_ref().map_or(0, |c| c.ammo),
			api_item3: value.context.as_ref().map_or(0, |c| c.steel),
			api_item4: value.context.as_ref().map_or(0, |c| c.bauxite),
			api_item5: value.context.as_ref().map_or(0, |c| c.devmat),
		}
	}
}
