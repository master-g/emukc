use crate::{
	kc2::start2::ApiManifest,
	thirdparty::{
		Kc3rdQuestConditionEquipInSlot, Kc3rdQuestConditionModelConversion,
		Kc3rdQuestConditionSlotItem, Kc3rdQuestConditionSlotItemType,
	},
};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionModelConversion {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let slots = if let Some(slots) = &self.slots {
			let slots =
				slots.iter().map(|slot| slot.to_json(mst)).collect::<Vec<serde_json::Value>>();
			Some(slots)
		} else {
			None
		};

		serde_json::json!({
			"slots": slots,
			"secretary": self.secretary.as_ref().map(|s| s.to_json(mst)),
			"banned": self.banned_secretary.as_ref().map(|s| s.to_json(mst)),
		})
	}
}

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionEquipInSlot {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		serde_json::json!({
			"item": self.item.to_json(mst),
			"pos": self.pos,
			"keep_stars": self.keep_stars,
		})
	}
}

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionSlotItem {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let items = match &self.item_type {
			Kc3rdQuestConditionSlotItemType::Equipment(id) => {
				mst.find_slotitem(*id).map(|item| item.api_name.clone()).unwrap_or_else(|| {
					error!("Unknown item ID: {}", id);
					"n/a".to_string()
				})
			}
			Kc3rdQuestConditionSlotItemType::Equipments(ids) => ids
				.iter()
				.map(|id| {
					mst.find_slotitem(*id).map(|item| item.api_name.clone()).unwrap_or_else(|| {
						error!("Unknown item ID: {}", id);
						"n/a".to_string()
					})
				})
				.collect::<Vec<String>>()
				.join(", "),
			Kc3rdQuestConditionSlotItemType::EquipType(id) => {
				mst.find_slotitem_type(*id).map(|item| item.api_name.clone()).unwrap_or_else(|| {
					error!("Unknown item type ID: {}", id);
					"n/a".to_string()
				})
			}
			Kc3rdQuestConditionSlotItemType::EquipTypes(ids) => ids
				.iter()
				.map(|id| {
					mst.find_slotitem_type(*id).map(|item| item.api_name.clone()).unwrap_or_else(
						|| {
							error!("Unknown item type ID: {}", id);
							"n/a".to_string()
						},
					)
				})
				.collect::<Vec<String>>()
				.join(", "),
		};

		serde_json::json!({
			"item_type": items,
			"amount": self.amount,
			"stars": self.stars,
			"fully_skilled": self.fully_skilled,
		})
	}
}
