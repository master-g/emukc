//! Test quest reward calculation

use std::path::Path;

use emukc::model::codex::Codex;
use emukc::{log::prelude::*, model::thirdparty::Kc3rdQuestRewardCategory};
use tracing::{debug, error, info, trace, warn};

fn load_codex() -> Codex {
	Codex::load(Path::new(".data/codex"), true).unwrap()
}

fn main() {
	new_log_builder().with_trace_level().build_simple();

	trace!("test");
	debug!("test");
	info!("test");
	warn!("test");
	error!("test");

	let codex = load_codex();
	for quest_manifest in codex.quest.values() {
		if let Some(from_id) = quest_manifest.get_model_conversion_info() {
			let from_slot_item = codex.manifest.find_slotitem(from_id);
			let from_name = from_slot_item.map(|v| v.api_name.as_str()).unwrap_or("unknown");

			println!(
				"quest {} {:?} {} {from_name}({from_id})",
				quest_manifest.api_no, quest_manifest.category, quest_manifest.name
			);

			for addition_reward in quest_manifest.additional_rewards.iter() {
				if matches!(addition_reward.category, Kc3rdQuestRewardCategory::Slotitem) {
					let name = codex
						.manifest
						.find_slotitem(addition_reward.api_id)
						.map(|v| v.api_name.as_str())
						.unwrap_or("unknown");
					println!("  additional reward: {name}({})", addition_reward.api_id);
				}
			}
			for choice_reward in quest_manifest.choice_rewards.iter() {
				for choice in choice_reward.choices.iter() {
					if matches!(choice.category, Kc3rdQuestRewardCategory::Slotitem) {
						let name = codex
							.manifest
							.find_slotitem(choice.api_id)
							.map(|v| v.api_name.as_str())
							.unwrap_or("unknown");
						println!("  choice reward: {name}({})", choice.api_id);
					}
				}
			}
		}
	}
}
