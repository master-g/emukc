use chrono::{DateTime, Utc};
use emukc_time::format_date;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::KcApiKDock;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum ConstructionDockStatus {
	/// Locked
	Locked = -1,
	/// Idle
	Idle = 0,
	/// In construction
	Busy = 1,
	/// Construction completed
	Completed = 2,
}

/// Construction context
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConstructionContext {
	/// Ship ID
	pub ship_id: i64,

	/// complete time
	pub complete_time: DateTime<Utc>,

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
			status: if index == 1 {
				ConstructionDockStatus::Idle
			} else {
				ConstructionDockStatus::Locked
			},
			context: None,
		})
	}

	/// Build API element
	pub fn build_api_element(&self) -> KcApiKDock {
		let api_complete_time_str = self
			.context
			.as_ref()
			.map_or("0".to_owned(), |c| format_date(c.complete_time.timestamp(), " "));
		KcApiKDock {
			api_id: self.index,
			api_state: self.status.clone() as i64,
			api_created_ship_id: self.context.as_ref().map_or(0, |c| c.ship_id),
			api_complete_time: self.context.as_ref().map_or(0, |c| c.complete_time.timestamp()),
			api_complete_time_str,
			api_item1: self.context.as_ref().map_or(0, |c| c.fuel),
			api_item2: self.context.as_ref().map_or(0, |c| c.ammo),
			api_item3: self.context.as_ref().map_or(0, |c| c.steel),
			api_item4: self.context.as_ref().map_or(0, |c| c.bauxite),
			api_item5: self.context.as_ref().map_or(0, |c| c.devmat),
		}
	}
}
