//! Test quest reward calculation

use std::path::Path;

use emukc::log::prelude::*;
use emukc::model::codex::Codex;
use emukc::model::thirdparty::{Kc3rdQuest, Kc3rdQuestCondition, Kc3rdQuestRequirement};
use tracing::warn;

fn load_codex() -> Codex {
	Codex::load(Path::new(".data/codex"), true).unwrap()
}

fn print_conversion_quests(codex: &Codex) {
	let mut model_conversion_quests = Vec::new();
	let mut item_conversion_quests = Vec::new();

	for quest_manifest in codex.quest.values() {
		if quest_manifest.has_slot_item_consumption() {
			if quest_manifest.has_slot_item_reward() {
				model_conversion_quests.push(quest_manifest);
			} else if quest_manifest.has_use_item_reward() {
				item_conversion_quests.push(quest_manifest);
			}
		}
	}

	println!("--- model conversion quests ---");
	for quest in model_conversion_quests {
		println!("model conversion quest {} {:?} {}", quest.api_no, quest.category, quest.name);
		if let Some((from_id, to_id)) = extract_model_conversion_info(quest) {
			let from_name = codex
				.manifest
				.find_slotitem(from_id)
				.map(|m| m.api_name.clone())
				.unwrap_or_else(|| format!("unknown slotitem {}", from_id));
			let to_name = codex
				.manifest
				.find_slotitem(to_id)
				.map(|m| m.api_name.clone())
				.unwrap_or_else(|| format!("unknown slotitem {}", to_id));
			println!("    converts model {from_name}({from_id}) to {to_name}({to_id})");
		} else {
			warn!("    no conversion info found");
		}
	}

	println!("--- item conversion quests ---");
	for quest in item_conversion_quests {
		println!("item conversion quest {} {:?} {}", quest.api_no, quest.category, quest.name);
	}
}

fn extract_model_conversion_info(quest: &Kc3rdQuest) -> Option<(i64, i64)> {
	let conditions: &Vec<Kc3rdQuestCondition> = match &quest.requirements {
		Kc3rdQuestRequirement::And(conds)
		| Kc3rdQuestRequirement::OneOf(conds)
		| Kc3rdQuestRequirement::Sequential(conds) => conds,
	};

	let mut from_id = 0;
	for cond in conditions {
		match cond {
			Kc3rdQuestCondition::ModelConversion(cond) => {
				if let Some(cond) = &cond.slots
					&& let Some(first) = cond.first()
				{
					match &first.item.item_type {
						emukc::model::thirdparty::Kc3rdQuestConditionSlotItemType::Equipment(
							id,
						) => {
							from_id = *id;
							break;
						}
						emukc::model::thirdparty::Kc3rdQuestConditionSlotItemType::Equipments(
							items,
						) => {
							if let Some(id) = items.first().copied() {
								from_id = id;
								break;
							}
						}
						_ => {}
					}
				}
			}
			Kc3rdQuestCondition::SlotItemConsumption(cond) => {
				if let Some(first) = cond.first() {
					match &first.item_type {
						emukc::model::thirdparty::Kc3rdQuestConditionSlotItemType::Equipment(
							id,
						) => {
							from_id = *id;
							break;
						}
						emukc::model::thirdparty::Kc3rdQuestConditionSlotItemType::Equipments(
							items,
						) => {
							if let Some(id) = items.first().copied() {
								from_id = id;
								break;
							}
						}
						_ => {}
					}
				}
			}
			_ => {}
		}
	}

	if from_id == 0 {
		return None;
	}

	for reward in quest.additional_rewards.iter() {
		if reward.category == emukc::model::thirdparty::Kc3rdQuestRewardCategory::Slotitem {
			return Some((from_id, reward.api_id));
		}
	}

	None
}

fn main() {
	new_log_builder().with_trace_level().build_simple();

	let codex = load_codex();
	print_conversion_quests(&codex);
}
