use serde::{Deserialize, Serialize};

use crate::KcApiMapRecord;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MapRecord {
	/// Profile ID
	pub id: i64,

	/// Map ID
	pub map_id: i64,

	/// Has cleared
	pub cleared: bool,

	/// Defeat count
	pub defeat_count: Option<i64>,

	/// Current map HP
	pub current_hp: Option<i64>,
}

impl MapRecord {
	/// Build API elements
	pub fn build_api_elements(&self) -> KcApiMapRecord {
		KcApiMapRecord {
			api_id: self.map_id,
			api_cleared: {
				if self.cleared {
					1
				} else {
					0
				}
			},
			api_defeat_count: self.defeat_count,
			api_now_maphp: self.current_hp,
		}
	}
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
