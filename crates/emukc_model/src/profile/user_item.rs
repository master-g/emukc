use serde::{Deserialize, Serialize};

use crate::kc2::KcApiUserItem;

/// User item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserItem {
	/// Profile ID
	pub id: i64,

	/// Manifest ID
	pub mst_id: i64,

	/// Item count
	pub count: i64,
}

impl From<UserItem> for KcApiUserItem {
	fn from(value: UserItem) -> Self {
		Self {
			api_id: value.mst_id,
			api_count: value.count,
		}
	}
}
