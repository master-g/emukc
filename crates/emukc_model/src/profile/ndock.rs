use chrono::{DateTime, Utc};
use emukc_time::format_date;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::KcApiNDock;

#[derive(Clone, Serialize, Deserialize, Debug)]
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
			status: if index == 1 {
				RepairDockStatus::Idle
			} else {
				RepairDockStatus::Locked
			},
			context: None,
		})
	}

	/// Build API element
	pub fn build_api_element(&self) -> KcApiNDock {
		let api_complete_time_str = self
			.context
			.as_ref()
			.map_or("0".to_owned(), |c| format_date(c.complete_time.timestamp(), " "));
		KcApiNDock {
			api_member_id: self.id,
			api_id: self.index,
			api_ship_id: self.context.as_ref().map_or(0, |c| c.ship_id),
			api_state: self.status.clone() as i64,
			api_complete_time: self.context.as_ref().map_or(0, |c| c.complete_time.timestamp()),
			api_complete_time_str,
			api_item1: self.context.as_ref().map_or(0, |c| c.fuel),
			api_item2: 0,
			api_item3: self.context.as_ref().map_or(0, |c| c.steel),
			api_item4: 0,
		}
	}
}
