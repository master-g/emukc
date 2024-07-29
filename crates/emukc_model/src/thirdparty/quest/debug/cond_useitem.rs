use crate::{start2::ApiManifest, Kc3rdQuestConditionUseItemConsumption};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionUseItemConsumption {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let m = mst.find_useitem(self.api_id).as_ref().map(|i| i.api_name.clone()).unwrap_or_else(
			|| {
				error!("Unknown useitem ID: {}", self.api_id);
				"n/a".to_string()
			},
		);

		serde_json::json!({
			"item": m,
			"amount": self.amount,
		})
	}
}
