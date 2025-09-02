//! Test quest reward calculation

use std::path::Path;

use emukc::model::codex::Codex;
use emukc::model::thirdparty::Kc3rdQuest;
use emukc::{log::prelude::*, model::thirdparty::Kc3rdQuestRewardCategory};

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
	}

	println!("--- item conversion quests ---");
	for quest in item_conversion_quests {
		println!("item conversion quest {} {:?} {}", quest.api_no, quest.category, quest.name);
	}
}

fn extract_model_conversion_info(quest: &Kc3rdQuest) -> Option<(i64, i64)> {
	None
}

fn main() {
	new_log_builder().with_trace_level().build_simple();

	let codex = load_codex();
	print_conversion_quests(&codex);
}
