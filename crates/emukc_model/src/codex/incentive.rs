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

	/// Create a new incentive with material as reward.
	///
	/// # Parameters
	///
	/// - `material_id`: The material manifest ID.
	/// - `amount`: The amount of material.
	pub fn new_incentive_with_material(
		&self,
		material_id: i64,
		amount: i64,
	) -> Result<KcApiIncentiveItem, CodexError> {
		Ok(KcApiIncentiveItem {
			api_mode: KcApiIncentiveMode::PreRegister as i64,
			api_type: KcApiIncentiveType::Resource as i64,
			api_mst_id: material_id,
			api_getmes: None,
			api_slotitem_level: None,
			amount,
			alv: 0,
		})
	}
}
