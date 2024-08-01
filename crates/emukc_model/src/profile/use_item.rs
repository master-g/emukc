use serde::{Deserialize, Serialize};

use crate::KcApiUserItem;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseItem {
	/// Profile ID
	pub id: i64,

	/// Item ID
	pub item_id: i64,

	/// Item count
	pub count: i64,
}

impl UseItem {
	/// Build API element
	pub fn build_api_element(&self) -> KcApiUserItem {
		KcApiUserItem {
			api_id: self.item_id,
			api_count: self.count,
		}
	}
}
