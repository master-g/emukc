use crate::{
	start2::ApiManifest, Kc3rdQuestConditionMapInfo, Kc3rdQuestConditionSortie,
	Kc3rdQuestConditionSortieMap,
};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionSortie {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		serde_json::json!({
			"composition": self.composition.as_ref().map(|c| c.to_json(mst)),
			"fleet_id": self.fleet_id,
			"defeat_boss": self.defeat_boss,
			"times": self.times,
			"result": self.result,
			"map": self.map.as_ref().map(|m| m.to_json(mst)),
		})
	}
}

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionSortieMap {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		match self {
			Kc3rdQuestConditionSortieMap::One(map) => {
				serde_json::json!({
					"type": "ONE",
					"map": map.to_json(mst),
				})
			}
			Kc3rdQuestConditionSortieMap::All(map) => {
				serde_json::json!({
					"type": "ALL",
					"maps": map.iter().map(|m| m.to_json(mst)).collect::<Vec<serde_json::Value>>(),
				})
			}
			Kc3rdQuestConditionSortieMap::AnyOf(map) => {
				serde_json::json!({
					"type": "ANY_OF",
					"maps": map.iter().map(|m| m.to_json(mst)).collect::<Vec<serde_json::Value>>(),
				})
			}
		}
	}
}

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionMapInfo {
	fn to_json(&self, _mst: &ApiManifest) -> serde_json::Value {
		serde_json::json!({
			"area": self.area,
			"number": self.number,
			"phase": self.phase,
		})
	}
}
