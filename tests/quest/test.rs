//! Test quest reward calculation

use std::path::Path;

use emukc::model::{
	codex::Codex,
	kc2::{KcApiQuestClearItemGet, KcApiQuestClearItemGetBonus},
};

fn load_codex() -> Codex {
	Codex::load(Path::new(".data/codex"), true).unwrap()
}

fn get_quest_rewards(
	codex: &Codex,
	quest_id: i64,
	choices: Option<Vec<i64>>,
) -> KcApiQuestClearItemGet {
	let quest_manifest = codex.quest.get(&quest_id).unwrap();
	let api_material = [
		quest_manifest.reward_fuel,
		quest_manifest.reward_ammo,
		quest_manifest.reward_steel,
		quest_manifest.reward_bauxite,
	];

	let mut api_bounus: Vec<KcApiQuestClearItemGetBonus> = quest_manifest
		.additional_rewards
		.iter()
		.map(|v| match v.category {
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::Material => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::Slotitem => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::Ship => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::Furniture => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::UseItem => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::FleetUnlock => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::LargeShipConstructionUnlock => {
				todo!()
			}
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::FactoryImprovementUnlock => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::WarResult => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::ExpeditionSupplyUnlock => todo!(),
			emukc::model::thirdparty::Kc3rdQuestRewardCategory::AirbaseUnlock => todo!(),
		})
		.collect();

	KcApiQuestClearItemGet {
		api_material,
		api_bounus_count: api_bounus.len() as i64,
		api_bounus,
	}
}

fn main() {
	let codex = load_codex();
	for quest_manifest in codex.quest.values() {}
}
