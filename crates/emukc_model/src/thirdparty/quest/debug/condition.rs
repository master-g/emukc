use crate::{start2::ApiManifest, Kc3rdQuestCondition, Kc3rdQuestConditionComposition};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestCondition {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		match self {
			Kc3rdQuestCondition::Composition(comp) => serde_json::json!({
				"type": "COMPOSITION",
				"composition": comp.to_json(mst),
			}),
			Kc3rdQuestCondition::Construct(n) => serde_json::json!({
				"type": "CONSTRUCT",
				"times": n,
			}),
			Kc3rdQuestCondition::Excercise(info) => serde_json::json!({
				"type": "EXCERCISE",
				"info": info.to_json(mst),
			}),
			Kc3rdQuestCondition::Expedition(info) => serde_json::json!({
				"type": "EXPEDITION",
				"info": info.iter().map(|i| i.to_json(mst)).collect::<Vec<serde_json::Value>>(),
			}),
			Kc3rdQuestCondition::ModelConversion(info) => serde_json::json!({
				"type": "MODEL_CONVERSION",
				"info": info.to_json(mst),
			}),
			Kc3rdQuestCondition::Modernization(info) => serde_json::json!({
				"type": "MODERNIZATION",
				"info": info.to_json(mst),
			}),
			Kc3rdQuestCondition::Repair(n) => serde_json::json!({
				"type": "REPAIR",
				"times": n,
			}),
			Kc3rdQuestCondition::ResourceConsumption(info) => serde_json::json! ({
				"type": "RESOURCE_CONSUMPTION",
				"resources": vec![info.fuel, info.ammo, info.steel, info.bauxite],
			}),
			Kc3rdQuestCondition::Resupply(n) => serde_json::json!({
				"type": "RESUPPLY",
				"times": n,
			}),
			Kc3rdQuestCondition::ScrapAnyEquipment(n) => serde_json::json!({
				"type": "SCRAP_ANY_EQUIPMENT",
				"times": n,
			}),
			Kc3rdQuestCondition::ScrapAnyShip(n) => serde_json::json!({
				"type": "SCRAP_ANY_SHIP",
				"times": n,
			}),
			Kc3rdQuestCondition::Sink(ship, amount) => serde_json::json!({
				"type": "SINK",
				"ships": ship.to_json(mst),
				"amount": amount,
			}),
			Kc3rdQuestCondition::SlotItemConstruction(n) => serde_json::json!({
				"type": "SLOT_ITEM_CONSTRUCTION",
				"times": n,
			}),
			Kc3rdQuestCondition::SlotItemConsumption(info) => serde_json::json!({
				"type": "SLOT_ITEM_CONSUMPTION",
				"consumes": info.iter().map(|i| i.to_json(mst)).collect::<Vec<serde_json::Value>>(),
			}),
			Kc3rdQuestCondition::SlotItemImprovement(n) => serde_json::json!({
				"type": "SLOT_ITEM_IMPROVEMENT",
				"times": n,
			}),
			Kc3rdQuestCondition::SlotItemScrap(info) => serde_json::json!({
				"type": "SLOT_ITEM_SCRAP",
				"scraps": info.iter().map(|i| i.to_json(mst)).collect::<Vec<serde_json::Value>>(),
			}),
			Kc3rdQuestCondition::Sortie(info) => serde_json::json!({
				"type": "SORTIE",
				"info": info.to_json(mst),
			}),
			Kc3rdQuestCondition::SortieCount(n) => serde_json::json!({
				"type": "SORTIE_COUNT",
				"times": n,
			}),
			Kc3rdQuestCondition::UseItemConsumption(items) => serde_json::json!({
				"type": "USE_ITEM_CONSUMPTION",
				"consumes": items.iter().map(|i| i.to_json(mst)).collect::<Vec<serde_json::Value>>(),
			}),
		}
	}
}

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionComposition {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let disallow = if let Some(disallowed) = &self.disallowed {
			let vec =
				disallowed.iter().map(|item| item.to_json(mst)).collect::<Vec<serde_json::Value>>();
			Some(vec)
		} else {
			None
		};

		let groups = self.groups.iter().map(|g| g.to_json(mst)).collect::<Vec<serde_json::Value>>();

		serde_json::json!({
			"fleet_id": self.fleet_id,
			"groups": groups,
			"disallowed": disallow,
		})
	}
}
