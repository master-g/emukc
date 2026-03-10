#![allow(missing_docs)]

use crate::profile::quest::QuestProgressStatus;

use super::{
	Kc3rdQuestCondition, Kc3rdQuestConditionFactory, Kc3rdQuestConditionScrap,
	Kc3rdQuestRequirement,
};

impl Kc3rdQuestRequirement {
	pub fn calculate_progress(&self, _mst: &Kc3rdQuestRequirement) -> QuestProgressStatus {
		match self {
			Kc3rdQuestRequirement::And(conditions) => {
				let total = conditions.len();
				let completed = conditions.iter().filter(|c| c.is_satisfied()).count();
				progress_from_ratio(completed, total)
			}
			Kc3rdQuestRequirement::OneOf(conditions) => {
				if conditions.iter().any(|c| c.is_satisfied()) {
					QuestProgressStatus::Completed
				} else {
					QuestProgressStatus::Empty
				}
			}
			Kc3rdQuestRequirement::Sequential(conditions) => {
				let first_incomplete = conditions.iter().position(|c| !c.is_satisfied());
				match first_incomplete {
					None => QuestProgressStatus::Completed,
					Some(idx) => progress_from_ratio(idx, conditions.len()),
				}
			}
		}
	}
}

impl Kc3rdQuestCondition {
	fn is_satisfied(&self) -> bool {
		match self {
			Kc3rdQuestCondition::Factory(f) => match f {
				Kc3rdQuestConditionFactory::ShipConstruction(count) => *count == 0,
				Kc3rdQuestConditionFactory::SlotItemConstruction(count) => *count == 0,
				_ => false,
			},
			Kc3rdQuestCondition::Scrap(s) => match s {
				Kc3rdQuestConditionScrap::AnyEquipment(count) => *count == 0,
				Kc3rdQuestConditionScrap::AnyShip(count) => *count == 0,
				_ => false,
			},
			Kc3rdQuestCondition::Repair(count) => *count == 0,
			Kc3rdQuestCondition::Resupply(count) => *count == 0,
			Kc3rdQuestCondition::Composition(_) => true,
			_ => false,
		}
	}
}

fn progress_from_ratio(completed: usize, total: usize) -> QuestProgressStatus {
	if total == 0 {
		return QuestProgressStatus::Completed;
	}
	let ratio = completed as f64 / total as f64;
	if ratio >= 1.0 {
		QuestProgressStatus::Completed
	} else if ratio >= 0.8 {
		QuestProgressStatus::Eighty
	} else if ratio >= 0.5 {
		QuestProgressStatus::Half
	} else {
		QuestProgressStatus::Empty
	}
}
