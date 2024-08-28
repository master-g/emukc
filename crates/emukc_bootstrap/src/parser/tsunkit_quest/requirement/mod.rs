mod conversion;
mod exchange;
mod exercise;
mod expedition;
mod modernization;
mod simple;
mod sortie;

use emukc_model::prelude::*;

use super::{ClassId, ConsumeCategory, Requirements, RequirementsCategory};

impl Requirements {
	pub(super) fn to_requirements(&self, mst: &ApiManifest) -> Kc3rdQuestRequirement {
		match self.category {
			RequirementsCategory::Or => Kc3rdQuestRequirement::OneOf(self.extract_conditions(mst)),
			RequirementsCategory::Then => {
				Kc3rdQuestRequirement::Sequential(self.extract_conditions(mst))
			}
			_ => Kc3rdQuestRequirement::And(self.extract_conditions(mst)),
		}
	}

	fn extract_conditions(&self, mst: &ApiManifest) -> Vec<Kc3rdQuestCondition> {
		match self.category {
			RequirementsCategory::And => self.extract_list(mst),
			RequirementsCategory::Conversion => self.extract_requirements_conversion(mst),
			RequirementsCategory::Equipexchange => self.extract_requirements_equip_exchange(mst),
			RequirementsCategory::Exercise => self.extract_requirements_exercise(mst),
			RequirementsCategory::Expedition => self.extract_requirements_expedition(),
			RequirementsCategory::Fleet => self.extract_requirements_fleet(mst),
			RequirementsCategory::Modernization => self.extract_requirements_modernization(mst),
			RequirementsCategory::Or => self.extract_list(mst),
			RequirementsCategory::Scrapequipment => self.extract_requirements_scrap_equipment(mst),
			RequirementsCategory::Simple => self.extract_requirements_simple(),
			RequirementsCategory::Sink => self.extract_requirements_sink(),
			RequirementsCategory::Sortie => self.extract_requirements_sortie(mst),
			RequirementsCategory::Then => self.extract_list(mst),
		}
	}

	fn extract_list(&self, mst: &ApiManifest) -> Vec<Kc3rdQuestCondition> {
		if let Some(list) = &self.list {
			list.iter()
				.flat_map(|item| Requirements::from(item.clone()).extract_conditions(mst))
				.collect()
		} else {
			vec![]
		}
	}

	pub(super) fn extract_resource_consumption(&self) -> Option<Kc3rdQuestCondition> {
		self.resources.as_ref().map(|resources| {
			Kc3rdQuestCondition::ResourceConsumption(Kc3rdQuestConditionMaterialConsumption {
				fuel: resources[0],
				ammo: resources[1],
				steel: resources[2],
				bauxite: resources[3],
			})
		})
	}

	fn extract_requirements_scrap_equipment(&self, mst: &ApiManifest) -> Vec<Kc3rdQuestCondition> {
		let list = if let Some(list) = &self.list {
			list
		} else {
			error!("scrap equipment requirement must have a list");
			return vec![];
		};

		let slotitems: Vec<Kc3rdQuestConditionSlotItem> = list
			.iter()
			.filter_map(|item| {
				let amount = item.amount.unwrap_or(1);
				let id = item.id.unwrap_or(0).abs(); // there are some negative ids in tsunkit db
				match item.category.as_str() {
					"equipment" => match mst.find_slotitem(id) {
						Some(mst) => {
							debug!("slot item found: {}, {}", mst.api_id, mst.api_name);
							Some(Kc3rdQuestConditionSlotItem {
								item_type: Kc3rdQuestConditionSlotItemType::Equipment(id),
								amount,
								stars: 0,
								fully_skilled: false,
							})
						}
						None => None,
					},
					"equipgroup" => match mst.find_slotitem_type(id) {
						Some(mst) => {
							debug!("slot item type found: {}, {}", mst.api_id, mst.api_name);
							Some(Kc3rdQuestConditionSlotItem {
								item_type: Kc3rdQuestConditionSlotItemType::EquipType(id),
								amount,
								stars: 0,
								fully_skilled: false,
							})
						}
						None => None,
					},
					_ => None,
				}
			})
			.collect();

		if slotitems.is_empty() {
			error!("scrap equipment requirement, no conditions found");
			return vec![];
		}

		let mut all: Vec<Kc3rdQuestCondition> = Vec::new();
		if let Some(res) = self.extract_resource_consumption() {
			all.push(res);
		}
		all.push(Kc3rdQuestCondition::SlotItemScrap(slotitems));

		all
	}

	pub(super) fn extract_useitem_consume(
		&self,
		mst: &ApiManifest,
	) -> Option<Vec<Kc3rdQuestConditionUseItemConsumption>> {
		if let Some(consume) = &self.consume {
			let consumptions: Vec<Kc3rdQuestConditionUseItemConsumption> = consume
				.iter()
				.filter_map(|c| {
					let api_id = if let Some(id) = c.id {
						id.abs()
					} else {
						error!("consume requirement must have an id");
						return None;
					};

					if let Some(category) = &c.category {
						match category {
							ConsumeCategory::Inventory => {
								if let Some(useitem_mst) = mst.find_useitem(api_id) {
									debug!(
										"use item found: {}, {}",
										useitem_mst.api_id, useitem_mst.api_name
									);
								} else {
									debug!("use item not found: {}", api_id);
									return None;
								}
								Some(Kc3rdQuestConditionUseItemConsumption {
									api_id,
									amount: c.amount,
								})
							}
							_ => None,
						}
					} else {
						None
					}
				})
				.collect();
			if consumptions.is_empty() {
				None
			} else {
				Some(consumptions)
			}
		} else {
			None
		}
	}

	pub(super) fn extract_slotitem_consume(
		&self,
		mst: &ApiManifest,
	) -> Option<Vec<Kc3rdQuestConditionSlotItem>> {
		if let Some(consume) = &self.consume {
			let consumptions: Vec<Kc3rdQuestConditionSlotItem> = consume
				.iter()
				.filter_map(|c| {
					let api_id = if let Some(id) = c.id {
						id.abs()
					} else {
						error!("consume requirement must have an id");
						return None;
					};

					let stars = c.stars.unwrap_or(0);

					if let Some(category) = &c.category {
						match category {
							ConsumeCategory::Equipgroup => {
								if let Some(equipgroup_mst) = mst.find_slotitem_type(api_id) {
									debug!(
										"slot item type found: {}, {}",
										equipgroup_mst.api_id, equipgroup_mst.api_name
									);
								} else {
									error!("slot item type not found: {}", api_id);
									return None;
								}
								Some(Kc3rdQuestConditionSlotItem {
									item_type: Kc3rdQuestConditionSlotItemType::EquipType(api_id),
									amount: c.amount,
									stars,
									fully_skilled: false,
								})
							}
							ConsumeCategory::Equipment => {
								if let Some(equipment_mst) = mst.find_slotitem(api_id) {
									debug!(
										"slot item found: {}, {}",
										equipment_mst.api_id, equipment_mst.api_name
									);
								} else {
									error!("slot item not found: {}", api_id);
									return None;
								}
								Some(Kc3rdQuestConditionSlotItem {
									item_type: Kc3rdQuestConditionSlotItemType::Equipment(api_id),
									amount: c.amount,
									stars,
									fully_skilled: false,
								})
							}
							_ => None,
						}
					} else {
						None
					}
				})
				.collect();
			if consumptions.is_empty() {
				None
			} else {
				Some(consumptions)
			}
		} else {
			None
		}
	}

	fn extract_requirements_sink(&self) -> Vec<Kc3rdQuestCondition> {
		let group_id = if let Some(group_id) = self.group_id {
			group_id
		} else {
			error!("sink requirement must have a group_id");
			return vec![];
		};

		let ship = if let Some(ship) = ClassId::find_ship_group(group_id) {
			ship
		} else {
			error!("ship group not found: {}", group_id);
			return vec![];
		};

		let amount = self.amount.unwrap_or(1);

		vec![Kc3rdQuestCondition::Sink(ship, amount)]
	}

	fn extract_requirements_fleet(&self, mst: &ApiManifest) -> Vec<Kc3rdQuestCondition> {
		let comp = match self.extract_fleet(mst) {
			Some(comp) => comp,
			None => {
				error!("fleet requirement must have a comp");
				return vec![];
			}
		};

		vec![Kc3rdQuestCondition::Composition(comp)]
	}

	fn extract_fleet(&self, mst: &ApiManifest) -> Option<Kc3rdQuestConditionComposition> {
		let fleet_id = self.fleet_id.unwrap_or(0);

		let groups: Vec<Kc3rdQuestConditionShipGroup> = if let Some(comp) = &self.comp {
			comp.iter().filter_map(|c| c.to_kc3rd_ship_group(mst)).collect()
		} else {
			return None;
		};

		let disallowed = if let Some(comp_banned) = &self.comp_banned {
			let mut banned: Vec<Kc3rdQuestConditionShip> = comp_banned
				.iter()
				.filter_map(|ban| {
					if let Some(class_id) = &ban.class_id {
						class_id.to_kc3rd_ship_class(mst)
					} else if let Some(ship_id) = &ban.ship_id {
						ship_id.to_kc3rd_ship_ids(mst)
					} else {
						None
					}
				})
				.collect();

			if let Some(extra_banned) = &self.disallowed {
				let ship = match extra_banned {
					super::Disallowed::Aviation => Some(Kc3rdQuestConditionShip::Aviation),
					super::Disallowed::Carriers => Some(Kc3rdQuestConditionShip::Carrier),
					_ => None,
				};
				if let Some(ship) = ship {
					banned.push(ship);
				}
			}

			Some(banned)
		} else {
			None
		};

		Some(Kc3rdQuestConditionComposition {
			groups,
			disallowed,
			fleet_id,
		})
	}
}
