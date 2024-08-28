use emukc_model::prelude::*;

use crate::parser::tsunkit_quest::Requirements;

impl Requirements {
	pub(super) fn extract_requirements_equip_exchange(
		&self,
		mst: &ApiManifest,
	) -> Vec<Kc3rdQuestCondition> {
		let mut conditions: Vec<Kc3rdQuestCondition> = Vec::new();

		// Extract resource consumption
		if let Some(res_consume) = self.extract_resource_consumption() {
			conditions.push(res_consume);
		}

		// Scrap
		if let Some((slot_items, use_items)) = self.extract_scrap(mst) {
			if !slot_items.is_empty() {
				conditions.push(Kc3rdQuestCondition::SlotItemScrap(slot_items));
			}
			if !use_items.is_empty() {
				conditions.push(Kc3rdQuestCondition::UseItemConsumption(use_items));
			}
		}

		// Consume
		if let Some(slotitem_consume) = self.extract_slotitem_consume(mst) {
			conditions.push(Kc3rdQuestCondition::SlotItemConsumption(slotitem_consume));
		}

		conditions
	}
}
