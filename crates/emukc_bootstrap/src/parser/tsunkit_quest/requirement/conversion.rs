use emukc_model::prelude::*;

use crate::parser::tsunkit_quest::{ConsumeCategory, Conversion, Requirements};

impl Requirements {
	pub(super) fn extract_requirements_conversion(
		&self,
		mst: &ApiManifest,
	) -> Vec<Kc3rdQuestCondition> {
		let mut conditions: Vec<Kc3rdQuestCondition> = Vec::new();

		// Extract resource consumption
		if let Some(condition) = self.extract_resource_consumption() {
			conditions.push(condition);
		}

		// Extract scrap
		if let Some((slot_items, use_items)) = self.extract_scrap(mst) {
			if !slot_items.is_empty() {
				conditions.push(Kc3rdQuestCondition::SlotItemScrap(slot_items));
			}
			if !use_items.is_empty() {
				conditions.push(Kc3rdQuestCondition::UseItemConsumption(use_items));
			}
		}

		// Extract secretary
		let secretary = self.secretary.as_ref().map(|secretary| {
			if let Some(class_id) = &secretary.class_id {
				Kc3rdQuestConditionShip::ShipTypes(class_id.clone())
			} else if let Some(family_id) = &secretary.family_id {
				family_id.to_kc3rd_ship_class(mst).unwrap_or(Kc3rdQuestConditionShip::Any)
			} else if let Some(ship_id) = &secretary.ship_id {
				ship_id.to_kc3rd_ship_ids(mst).unwrap_or(Kc3rdQuestConditionShip::Any)
			} else {
				Kc3rdQuestConditionShip::Any
			}
		});

		// Extract banned secretary
		let banned_secretary = self
			.secretary_banned
			.as_ref()
			.map(|v| Kc3rdQuestConditionShip::Ships(v.ship_id.clone()));

		// Extract slots
		let slots = self.extract_slots(mst);

		// Extract consume
		if let Some(useitem_consume) = self.extract_useitem_consume(mst) {
			conditions.push(Kc3rdQuestCondition::UseItemConsumption(useitem_consume));
		}
		if let Some(slotitem_consume) = self.extract_slotitem_consume(mst) {
			conditions.push(Kc3rdQuestCondition::SlotItemConsumption(slotitem_consume));
		}

		if secretary.is_none() && banned_secretary.is_none() && slots.is_none() {
			warn!("no conversion conditions found, ignore");
		} else {
			conditions.push(Kc3rdQuestCondition::ModelConversion(
				Kc3rdQuestConditionModelConversion {
					secretary,
					banned_secretary,
					slots,
				},
			));
		}

		conditions
	}

	fn extract_slots(&self, mst: &ApiManifest) -> Option<Vec<Kc3rdQuestConditionEquipInSlot>> {
		if let Some(slots) = &self.slots {
			let slots: Vec<Kc3rdQuestConditionEquipInSlot> = slots
				.iter()
				.filter_map(|s| {
					let stars = s.stars.unwrap_or(0);
					let api_id = if let Some(slotitem_mst) = mst.find_slotitem(s.id) {
						debug!(
							"slot item found: {}, {}",
							slotitem_mst.api_id, slotitem_mst.api_name
						);
						s.id
					} else {
						error!("slot item not found: {}", s.id);
						return None;
					};
					let fully_skilled = s.fullyskilled.unwrap_or(false);
					let pos = s.slot.unwrap_or(0);
					let keep_stars = match &s.conversion {
						Some(Conversion::Starskept) => true,
						_ => false,
					};
					Some(Kc3rdQuestConditionEquipInSlot {
						item: Kc3rdQuestConditionSlotItem {
							item_type: Kc3rdQuestConditionSlotItemType::Equipment(api_id),
							amount: 1,
							stars,
							fully_skilled,
						},
						pos,
						keep_stars,
					})
				})
				.collect();
			if slots.is_empty() {
				None
			} else {
				Some(slots)
			}
		} else {
			None
		}
	}

	pub(super) fn extract_scrap(
		&self,
		mst: &ApiManifest,
	) -> Option<(Vec<Kc3rdQuestConditionSlotItem>, Vec<Kc3rdQuestConditionUseItemConsumption>)> {
		if let Some(scrap) = &self.scrap {
			let slot_items: Vec<Kc3rdQuestConditionSlotItem> = scrap
				.iter()
				.filter_map(|s| {
					let amount = s.amount;
					let id = s.id.abs();
					let item_type = match &s.category {
						ConsumeCategory::Equipgroup => {
							if let Some(m) = mst.find_slotitem_type(id) {
								debug!("slot item type found: {}, {}", m.api_id, m.api_name);
							} else {
								error!("slot item type not found: {}", id);
								return None;
							}
							Kc3rdQuestConditionSlotItemType::EquipType(id)
						}
						ConsumeCategory::Equipment => {
							if let Some(m) = mst.find_slotitem(id) {
								debug!("slot item found: {}, {}", m.api_id, m.api_name);
							} else {
								error!("slot item not found: {}", id);
								return None;
							}
							Kc3rdQuestConditionSlotItemType::Equipment(id)
						}
						ConsumeCategory::Inventory => return None,
					};
					Some(Kc3rdQuestConditionSlotItem {
						item_type,
						amount,
						stars: 0,
						fully_skilled: false,
					})
				})
				.collect();

			let use_items: Vec<Kc3rdQuestConditionUseItemConsumption> = scrap
				.iter()
				.filter_map(|s| {
					let amount = s.amount;
					let id = s.id.abs();
					let item_type = match &s.category {
						ConsumeCategory::Equipgroup => return None,
						ConsumeCategory::Equipment => return None,
						ConsumeCategory::Inventory => {
							if let Some(m) = mst.find_useitem(id) {
								debug!("use item found: {}, {}", m.api_id, m.api_name);
							} else {
								error!("use item not found: {}", id);
								return None;
							}
							Kc3rdQuestConditionUseItemConsumption {
								api_id: id,
								amount,
							}
						}
					};
					Some(item_type)
				})
				.collect();

			if slot_items.is_empty() && use_items.is_empty() {
				None
			} else {
				Some((slot_items, use_items))
			}
		} else {
			None
		}
	}
}
