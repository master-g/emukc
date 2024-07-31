use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::KcApiDeckPort;

/// Fleet mission status
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum FleetMissionStatus {
	Idle = 0,
	InMission = 1,
	Returning = 2,
	ForceReturning = 3,
}

/// Fleet mission context
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FleetMissionContext {
	/// mission ID
	pub id: i64,

	/// status
	pub status: FleetMissionStatus,

	/// mission return time
	pub return_time: Option<DateTime<Utc>>,
}

/// Fleet
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Fleet {
	/// Profile ID
	pub id: i64,

	/// Fleet ID, 1-4
	pub index: i64,

	/// Fleet name
	pub name: String,

	/// Mission status
	pub mission: Option<FleetMissionContext>,

	/// Fleet ships, length is always 6, empty slot is filled with -1
	pub ships: Vec<i64>,
}

/// Fleet error
#[derive(Error, Debug)]
pub enum FleetError {
	/// Fleet ID out of range
	#[error("Fleet ID out of range: {0}")]
	OutOfRange(i64),
}

/// Fleet implementation
impl Fleet {
	/// Create a new fleet
	/// Note that the ships are not initialized
	///
	/// # Arguments
	///
	/// * `id` - Profile ID
	/// * `index` - Fleet ID, 1-4
	///
	/// # Returns
	///
	/// * Fleet instance
	///
	/// # Errors
	///
	/// * If the fleet ID is out of range
	pub fn new(id: i64, index: i64) -> Result<Self, FleetError> {
		if !(1..=4).contains(&index) {
			return Err(FleetError::OutOfRange(index));
		}

		let name = format!("\u{7b2c} {} \u{8266}\u{968a}", id);

		Ok(Self {
			id,
			index,
			name,
			mission: None,
			ships: Vec::new(),
		})
	}

	/// Build API element
	pub fn build_api_element(&self) -> KcApiDeckPort {
		KcApiDeckPort {
			api_member_id: self.id,
			api_id: self.index,
			api_name: self.name.to_owned(),
			api_name_id: "".to_owned(),
			api_mission: vec![0; 4],
			api_flagship: "0".to_owned(),
			api_ship: self.ships.clone(),
		}
	}
}
