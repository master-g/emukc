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
                vec![Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
                    times,
                    ..Default::default()
                })]
            }
            RequirementsSubCategory::Equipment => {
                vec![Kc3rdQuestCondition::Factory(
                    Kc3rdQuestConditionFactory::SlotItemConstruction(times),
                )]
            }
            RequirementsSubCategory::Improvement => {
                vec![Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(
                    times,
                ))]
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
                vec![Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyEquipment(times))]
            }
            RequirementsSubCategory::Scrapship => {
                vec![Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(times))]
            }
            RequirementsSubCategory::Ship => {
                vec![Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(
                    times,
                ))]
            }
        }
    }
}
