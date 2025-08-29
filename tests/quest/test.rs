//! Test quest reward calculation

use std::path::Path;

use emukc::{
	gameplay::prelude::*,
	model::{
		codex::Codex,
		kc2::{
			KcApiQuestClearItemBonusType, KcApiQuestClearItemGet, KcApiQuestClearItemGetBonus,
			KcApiQuestClearItemGetBonusItem,
		},
		profile::fleet::Fleet,
	},
};

fn load_codex() -> Codex {
	Codex::load(Path::new(".data/codex"), true).unwrap()
}

fn main() {
	let codex = load_codex();
	for quest_manifest in codex.quest.values() {}
}
