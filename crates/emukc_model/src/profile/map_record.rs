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
