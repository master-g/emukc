use crate::{kc2::start2::ApiManifest, thirdparty::Kc3rdQuestRequirement};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestRequirement {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let conds = match self {
			Kc3rdQuestRequirement::Sequential(conds)
			| Kc3rdQuestRequirement::OneOf(conds)
			| Kc3rdQuestRequirement::And(conds) => conds,
		};

		let conds = conds.iter().map(|cond| cond.to_json(mst)).collect::<Vec<serde_json::Value>>();

		match self {
			Kc3rdQuestRequirement::And(_) => serde_json::json!({
				"type": "AND",
				"conds": conds,
			}),
			Kc3rdQuestRequirement::OneOf(_) => serde_json::json!({
				"type": "ONE_OF",
				"conds": conds,
			}),
			Kc3rdQuestRequirement::Sequential(_) => serde_json::json!({
				"type": "SEQUENTIAL",
				"conds": conds,
			}),
		}
	}
}
