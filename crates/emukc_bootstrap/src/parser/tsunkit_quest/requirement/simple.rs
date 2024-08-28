use emukc_model::prelude::*;

use crate::parser::tsunkit_quest::{Requirements, RequirementsSubCategory};

impl Requirements {
	pub(super) fn extract_requirements_simple(&self) -> Vec<Kc3rdQuestCondition> {
		let Some(subcategory) = &self.subcategory else {
			error!("simple requirement must have a subcategory");
			return vec![];
		};

		let times = self.times.unwrap_or(0);

		match subcategory {
			RequirementsSubCategory::Battle => {
				vec![Kc3rdQuestCondition::SortieCount(times)]
			}
			RequirementsSubCategory::Equipment => {
				vec![Kc3rdQuestCondition::SlotItemConstruction(times)]
			}
			RequirementsSubCategory::Improvement => {
				vec![Kc3rdQuestCondition::SlotItemImprovement(times)]
			}
			RequirementsSubCategory::Modernization => {
				vec![Kc3rdQuestCondition::Modernization(Kc3rdQuestConditionModernization {
					target_ship: Kc3rdQuestConditionShip::Any,
					material_ship: Kc3rdQuestConditionShip::Any,
					batch_size: 1,
					times,
				})]
			}
			RequirementsSubCategory::Repair => {
				vec![Kc3rdQuestCondition::Repair(times)]
			}
			RequirementsSubCategory::Resupply => {
				vec![Kc3rdQuestCondition::Resupply(times)]
			}
			RequirementsSubCategory::Scrapequipment => {
				vec![Kc3rdQuestCondition::ScrapAnyEquipment(times)]
			}
			RequirementsSubCategory::Scrapship => {
				vec![Kc3rdQuestCondition::ScrapAnyShip(times)]
			}
			RequirementsSubCategory::Ship => {
				vec![Kc3rdQuestCondition::Construct(times)]
			}
		}
	}
}
