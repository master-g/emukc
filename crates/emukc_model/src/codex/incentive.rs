//! incentive extension for `Codex`

use crate::kc2::{KcApiIncentiveItem, KcApiIncentiveMode, KcApiIncentiveType};

use super::{Codex, CodexError};

impl Codex {
	/// Create a new incentive with ship as reward.
	///
	/// # Parameters
	///
	/// - `ship_id`: The ship manifest ID.
	pub fn new_incentive_with_ship(&self, ship_id: i64) -> Result<KcApiIncentiveItem, CodexError> {
		Ok(KcApiIncentiveItem {
			api_mode: KcApiIncentiveMode::PreRegister as i64,
			api_type: KcApiIncentiveType::Ship as i64,
			api_mst_id: ship_id,
			api_getmes: None,
			api_slotitem_level: None,
			amount: 0,
			alv: 0,
		})
	}
}
