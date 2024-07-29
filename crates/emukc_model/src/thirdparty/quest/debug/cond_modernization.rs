use crate::{start2::ApiManifest, Kc3rdQuestConditionModernization};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionModernization {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		serde_json::json!({
			"target": self.target_ship.to_json(mst),
			"material": self.material_ship.to_json(mst),
			"batch_size": self.batch_size,
			"times": self.times,
		})
	}
}
