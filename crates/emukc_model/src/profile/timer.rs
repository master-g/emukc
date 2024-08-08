use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// User timers
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct UserTimer {
	/// profile id
	pub id: i64,
	/// first resource timer
	pub primary_resource: DateTime<Utc>,
	/// second resource timer
	pub bauxite: DateTime<Utc>,
	/// repair dock timer
	pub repair_dock: DateTime<Utc>,
	/// quest timer
	pub quest: DateTime<Utc>,
	/// pratice rival timer
	pub rival: DateTime<Utc>,
}

impl Default for UserTimer {
	fn default() -> Self {
		Self {
			id: 0,
			primary_resource: Utc::now(),
			bauxite: Utc::now(),
			repair_dock: Utc::now(),
			quest: Utc::now(),
			rival: Utc::now(),
		}
	}
}
