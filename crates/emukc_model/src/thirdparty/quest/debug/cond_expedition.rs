use crate::{kc2::start2::ApiManifest, thirdparty::Kc3rdQuestConditionExpedition};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionExpedition {
	fn to_json(&self, _mst: &ApiManifest) -> serde_json::Value {
		serde_json::json!({
			"times": self.times,
			"list": self.list,
		})
	}
}
