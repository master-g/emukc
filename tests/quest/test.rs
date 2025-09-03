//! Test quest reward calculation

use std::path::Path;

use emukc::model::codex::Codex;
use emukc::model::thirdparty::reward::get_quest_rewards;
use emukc::{log::prelude::*, model::thirdparty::Kc3rdQuest};
use tracing::{debug, error, info, trace, warn};

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
		if let Some((from_id, to_id)) = quest.extract_model_conversion_info() {
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

fn dump_all_model_conversion_quest_reward_api_response(codex: &Codex) {
	let mut aircraft_conversion_quests = Vec::new();
	let mut other_conversion_quests = Vec::new();
	for quest in codex.quest.values() {
		if quest.has_slot_item_consumption() && quest.has_slot_item_reward() {
			if let Some((from_id, to_id)) = quest.extract_model_conversion_info() {
				if let Some(from) = codex.manifest.find_slotitem(from_id)
					&& let Some(to) = codex.manifest.find_slotitem(to_id)
				{
					if from.api_type[4] != 0 && to.api_type[4] != 0 {
						aircraft_conversion_quests.push(quest);
						continue;
					}
				}
			}

			other_conversion_quests.push(quest);
		}
	}

	let print_quest = |quest: &Kc3rdQuest| {
		println!("{} {}", quest.api_no, quest.name);

		let choices = if quest.choice_rewards.is_empty() {
			None
		} else {
			let list: Vec<i64> = quest.choice_rewards.iter().map(|_| 0).collect();
			Some(list)
		};

		match get_quest_rewards(codex, quest.api_no, choices) {
			Ok(resp) => {
				// let json = serde_json::to_string(&resp).unwrap();
				// println!("{}\n", json);
				for bonus in resp.api_bounus.iter() {
					if let Some(item) = &bonus.api_item {
						if let Some(msg) = &item.api_message {
							println!("   {msg}");
						}
					}
				}
			}
			Err(e) => {
				error!("  failed to get rewards: {}", e);
			}
		}
	};

	for quest in aircraft_conversion_quests {
		print_quest(quest);
	}

	println!("\n\n\n");

	for quest in other_conversion_quests {
		print_quest(quest);
	}
}

fn main() {
	new_log_builder().with_log_level("info").build_simple();

	let codex = load_codex();
	// print_conversion_quests(&codex);
	dump_all_model_conversion_quest_reward_api_response(&codex);
}
