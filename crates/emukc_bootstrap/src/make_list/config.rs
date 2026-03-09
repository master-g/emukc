use serde::{Deserialize, Serialize};

/// Greedy mode configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GreedyConfig {
	/// Number of concurrent checks
	pub concurrent: usize,
}

impl Default for GreedyConfig {
	fn default() -> Self {
		Self {
			concurrent: 16,
		}
	}
}
