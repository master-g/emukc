use serde::{Deserialize, Serialize};

/// User world record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KcUserWorld {
	pub world: i64,
}
