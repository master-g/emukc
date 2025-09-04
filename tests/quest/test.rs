//! Test quest reward calculation

use std::path::Path;

use emukc::model::codex::Codex;
use emukc::model::thirdparty::Kc3rdQuestConversionMode;
use emukc::model::thirdparty::reward::get_quest_rewards;
use emukc::{log::prelude::*, model::thirdparty::Kc3rdQuest};
use tracing::{debug, error, info, trace, warn};

fn load_codex() -> Codex {
	Codex::load(Path::new(".data/codex"), true).unwrap()
}

fn dump_all_model_conversion_quest_reward_api_response(codex: &Codex) {
	codex
		.quest
		.values()
		.filter(|v| {
			!matches!(v.conversion_mode, Kc3rdQuestConversionMode::None)
				&& !v.additional_rewards.is_empty()
		})
		.for_each(|quest| {
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
		});
}

fn dump2(codex: &Codex) {
	let mut conversion_quests = Vec::new();
	let mut exchange_quests = Vec::new();
	for (_, quest) in codex.quest.iter() {
		if quest.additional_rewards.is_empty() {
			continue;
		}
		match quest.conversion_mode {
			Kc3rdQuestConversionMode::Conversion => {
				conversion_quests.push(quest);
			}
			Kc3rdQuestConversionMode::Exchange => {
				exchange_quests.push(quest);
			}
			Kc3rdQuestConversionMode::None => {}
		}
	}

	println!("--- conversion quests ---");
	for quest in conversion_quests {
		println!("conversion quest {} {}", quest.api_no, quest.name);
	}

	println!("--- exchange quests ---");
	for quest in exchange_quests {
		println!("exchange quest {} {}", quest.api_no, quest.name);
	}
}

fn main() {
	new_log_builder().with_log_level("trace").build_simple();

	let codex = load_codex();
	dump_all_model_conversion_quest_reward_api_response(&codex);
	println!("\n\n\n\n\n\n\n\n");
	dump2(&codex);
}
