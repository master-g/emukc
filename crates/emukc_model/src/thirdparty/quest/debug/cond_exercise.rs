use crate::{kc2::start2::ApiManifest, thirdparty::Kc3rdQuestConditionExcerise};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionExcerise {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let groups = self.groups.as_ref().map(|groups| {
			groups.iter().map(|g| g.to_json(mst)).collect::<Vec<serde_json::Value>>()
		});

		serde_json::json!({
			"times": self.times,
			"expect": self.expect_result,
			"daily": self.expire_next_day,
			"groups": groups,
		})
	}
}
