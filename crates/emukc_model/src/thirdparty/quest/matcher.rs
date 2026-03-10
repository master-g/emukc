//! Quest condition matching

use crate::thirdparty::{
	Kc3rdQuestCondition, Kc3rdQuestConditionFactory, Kc3rdQuestConditionScrap,
};

/// Quest action events
#[derive(Debug, Clone)]
pub enum QuestActionEvent {
	ShipConstructed {
		ship_mst_id: i64,
		large: bool,
	},
	SlotItemConstructed {
		item_mst_id: i64,
	},
	ShipScrapped {
		ship_mst_id: i64,
	},
	SlotItemScrapped {
		item_mst_id: i64,
	},
	ShipRepaired {
		ship_id: i64,
	},
	ShipResupplied {
		ship_id: i64,
	},
}

impl Kc3rdQuestCondition {
	pub fn matches_event(&self, event: &QuestActionEvent) -> bool {
		match (self, event) {
			(
				Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(_)),
				QuestActionEvent::ShipConstructed {
					..
				},
			) => true,
			(
				Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemConstruction(_)),
				QuestActionEvent::SlotItemConstructed {
					..
				},
			) => true,
			(
				Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(_)),
				QuestActionEvent::ShipScrapped {
					..
				},
			) => true,
			(
				Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyEquipment(_)),
				QuestActionEvent::SlotItemScrapped {
					..
				},
			) => true,
			(
				Kc3rdQuestCondition::Repair(_),
				QuestActionEvent::ShipRepaired {
					..
				},
			) => true,
			(
				Kc3rdQuestCondition::Resupply(_),
				QuestActionEvent::ShipResupplied {
					..
				},
			) => true,
			_ => false,
		}
	}

	pub fn apply_event(&mut self, event: &QuestActionEvent) -> bool {
		if !self.matches_event(event) {
			return false;
		}

		match self {
			Kc3rdQuestCondition::Factory(f) => match f {
				Kc3rdQuestConditionFactory::ShipConstruction(count)
				| Kc3rdQuestConditionFactory::SlotItemConstruction(count) => {
					if *count > 0 {
						*count -= 1;
						true
					} else {
						false
					}
				}
				_ => false,
			},
			Kc3rdQuestCondition::Scrap(s) => match s {
				Kc3rdQuestConditionScrap::AnyShip(count)
				| Kc3rdQuestConditionScrap::AnyEquipment(count) => {
					if *count > 0 {
						*count -= 1;
						true
					} else {
						false
					}
				}
				_ => false,
			},
			Kc3rdQuestCondition::Repair(count) | Kc3rdQuestCondition::Resupply(count) => {
				if *count > 0 {
					*count -= 1;
					true
				} else {
					false
				}
			}
			_ => false,
		}
	}
}
