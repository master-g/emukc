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
	ExpeditionCompleted {
		mission_id: i64,
		result: ExpeditionResult,
		fleet_id: i64,
	},
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpeditionResult {
	Failure = 0,
	Success = 1,
	GreatSuccess = 2,
}

impl Kc3rdQuestCondition {
	pub fn matches_event(&self, event: &QuestActionEvent) -> bool {
		matches!(
			(self, event),
			(
				Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(_)),
				QuestActionEvent::ShipConstructed { .. },
			) | (
				Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemConstruction(_)),
				QuestActionEvent::SlotItemConstructed { .. },
			) | (
				Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(_)),
				QuestActionEvent::ShipScrapped { .. },
			) | (
				Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyEquipment(_)),
				QuestActionEvent::SlotItemScrapped { .. },
			) | (Kc3rdQuestCondition::Repair(_), QuestActionEvent::ShipRepaired { .. },)
				| (Kc3rdQuestCondition::Resupply(_), QuestActionEvent::ShipResupplied { .. },)
				| (
					Kc3rdQuestCondition::Expedition(_),
					QuestActionEvent::ExpeditionCompleted { .. },
				)
		)
	}

	pub fn apply_event(&mut self, event: &QuestActionEvent) -> bool {
		if !self.matches_event(event) {
			return false;
		}

		match self {
			Kc3rdQuestCondition::Factory(
				Kc3rdQuestConditionFactory::ShipConstruction(count)
				| Kc3rdQuestConditionFactory::SlotItemConstruction(count),
			)
			| Kc3rdQuestCondition::Scrap(
				Kc3rdQuestConditionScrap::AnyShip(count)
				| Kc3rdQuestConditionScrap::AnyEquipment(count),
			)
			| Kc3rdQuestCondition::Repair(count)
			| Kc3rdQuestCondition::Resupply(count) => {
				if *count > 0 {
					*count -= 1;
					true
				} else {
					false
				}
			}
			Kc3rdQuestCondition::Expedition(conditions) => {
				let QuestActionEvent::ExpeditionCompleted {
					mission_id,
					..
				} = event
				else {
					return false;
				};

				for condition in conditions.iter_mut() {
					let matches = condition.list.as_ref().is_none_or(|allowed_ids| {
						let mission_id_str = mission_id.to_string();
						allowed_ids.iter().any(|id| {
							id == &mission_id_str
								|| id.parse::<i64>().ok().is_some_and(|v| v == *mission_id)
						})
					});

					if matches && condition.times > 0 {
						condition.times -= 1;
						return true;
					}
				}

				false
			}
			_ => false,
		}
	}
}
