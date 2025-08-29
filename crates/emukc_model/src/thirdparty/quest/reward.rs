use thiserror::Error;

use crate::{
	codex::Codex,
	kc2::{
		KcApiQuestClearItemBonusType, KcApiQuestClearItemGet, KcApiQuestClearItemGetBonus,
		KcApiQuestClearItemGetBonusItem, KcUseItemType, MaterialCategory,
	},
	prelude::{ApiManifest, Kc3rdQuestReward, Kc3rdQuestRewardCategory},
	profile::fleet::{Fleet, FleetError},
};

#[derive(Error, Debug, Clone)]
pub enum RewardError {
	/// Invalid fleet info in the quest rewards
	#[error(transparent)]
	InvalidFleet(#[from] FleetError),

	/// Invalid ship info in the quest rewards
	#[error("invalid ship id {0}")]
	InvalidShip(i64),
}

/// convert Kc3rdQuestReward to KcApiQuestClearItemGetBonus for most cases
/// except for model conversion
fn convert_kc3rd_quest_reward_to_api(
	manifest: &ApiManifest,
	reward: &Kc3rdQuestReward,
) -> Result<Option<KcApiQuestClearItemGetBonus>, RewardError> {
	let bonus = match reward.category {
		Kc3rdQuestRewardCategory::Material => Some(KcApiQuestClearItemGetBonus {
			api_type: KcApiQuestClearItemBonusType::Material as i64,
			api_count: reward.amount,
			api_item: Some(KcApiQuestClearItemGetBonusItem {
				api_id: Some(reward.api_id),
				..Default::default()
			}),
		}),
		Kc3rdQuestRewardCategory::Slotitem => Some(KcApiQuestClearItemGetBonus {
			api_type: KcApiQuestClearItemBonusType::SlotItem as i64,
			api_count: reward.amount,
			api_item: Some(KcApiQuestClearItemGetBonusItem {
				api_id: Some(reward.api_id),
				api_slotitem_level: (reward.stars > 0).then_some(reward.stars),
				..Default::default()
			}),
		}),
		Kc3rdQuestRewardCategory::Ship => {
			let ship_mst =
				manifest.find_ship(reward.api_id).ok_or(RewardError::InvalidShip(reward.api_id))?;

			Some(KcApiQuestClearItemGetBonus {
				api_type: KcApiQuestClearItemBonusType::ShipBonus as i64,
				api_count: reward.amount,
				api_item: Some(KcApiQuestClearItemGetBonusItem {
					api_ship_id: Some(reward.api_id),
					api_name: Some(ship_mst.api_name.clone()),
					api_getmes: ship_mst.api_getmes.clone(),
					..Default::default()
				}),
			})
		}
		Kc3rdQuestRewardCategory::Furniture => Some(KcApiQuestClearItemGetBonus {
			api_type: KcApiQuestClearItemBonusType::Furniture as i64,
			api_count: 1,
			api_item: Some(KcApiQuestClearItemGetBonusItem {
				api_id: Some(reward.api_id),
				..Default::default()
			}),
		}),
		Kc3rdQuestRewardCategory::UseItem => match KcUseItemType::n(reward.api_id) {
			Some(typ) => {
				let (api_type, real_id) = match typ {
					KcUseItemType::Bucket => {
						(KcApiQuestClearItemBonusType::Material, MaterialCategory::Bucket as i64)
					}
					KcUseItemType::Torch => {
						(KcApiQuestClearItemBonusType::Material, MaterialCategory::Torch as i64)
					}
					KcUseItemType::DevMaterial => {
						(KcApiQuestClearItemBonusType::Material, MaterialCategory::DevMat as i64)
					}
					KcUseItemType::Screw => {
						(KcApiQuestClearItemBonusType::Material, MaterialCategory::Screw as i64)
					}
					KcUseItemType::Fuel => {
						(KcApiQuestClearItemBonusType::Material, MaterialCategory::Fuel as i64)
					}
					KcUseItemType::Ammo => {
						(KcApiQuestClearItemBonusType::Material, MaterialCategory::Ammo as i64)
					}
					KcUseItemType::Steel => {
						(KcApiQuestClearItemBonusType::Material, MaterialCategory::Steel as i64)
					}
					KcUseItemType::Bauxite => {
						(KcApiQuestClearItemBonusType::Material, MaterialCategory::Bauxite as i64)
					}
					KcUseItemType::FCoinBox200 => (
						KcApiQuestClearItemBonusType::FurnitureCoinBox,
						KcUseItemType::FCoinBox200 as i64,
					),
					KcUseItemType::FCoinBox400 => (
						KcApiQuestClearItemBonusType::FurnitureCoinBox,
						KcUseItemType::FCoinBox400 as i64,
					),
					KcUseItemType::FCoinBox700 => (
						KcApiQuestClearItemBonusType::FurnitureCoinBox,
						KcUseItemType::FCoinBox700 as i64,
					),
					_ => (KcApiQuestClearItemBonusType::UseItem, reward.api_id),
				};
				Some(KcApiQuestClearItemGetBonus {
					api_type: api_type as i64,
					api_count: reward.amount,
					api_item: Some(KcApiQuestClearItemGetBonusItem {
						api_id: Some(real_id),
						..Default::default()
					}),
				})
			}
			_ => Some(KcApiQuestClearItemGetBonus {
				api_type: KcApiQuestClearItemBonusType::UseItem as i64,
				api_count: reward.amount,
				api_item: Some(KcApiQuestClearItemGetBonusItem {
					api_id: Some(reward.api_id),
					..Default::default()
				}),
			}),
		},
		Kc3rdQuestRewardCategory::FleetUnlock => {
			let fleet = Fleet::new(0, reward.api_id)?;
			Some(KcApiQuestClearItemGetBonus {
				api_type: KcApiQuestClearItemBonusType::UnlockDeck as i64,
				api_count: 1,
				api_item: Some(KcApiQuestClearItemGetBonusItem {
					api_id: Some(reward.api_id),
					api_name: Some(fleet.name),
					..Default::default()
				}),
			})
		}
		Kc3rdQuestRewardCategory::LargeShipConstructionUnlock => {
			Some(KcApiQuestClearItemGetBonus {
				api_type: KcApiQuestClearItemBonusType::UnlockLargeBuild as i64,
				api_count: 0,
				api_item: None,
			})
		}
		Kc3rdQuestRewardCategory::WarResult => Some(KcApiQuestClearItemGetBonus {
			api_type: KcApiQuestClearItemBonusType::WarResult as i64,
			api_count: reward.amount,
			api_item: None,
		}),
		Kc3rdQuestRewardCategory::ExpeditionSupplyUnlock => Some(KcApiQuestClearItemGetBonus {
			api_type: KcApiQuestClearItemBonusType::ExtraSupply as i64,
			api_count: 0,
			api_item: None,
		}),
		Kc3rdQuestRewardCategory::AirbaseUnlock => {
			let name = match reward.api_id {
				6 => "中部海域",
				7 => "南西海域",
				_ => "fixme: unknow airbase",
			};
			Some(KcApiQuestClearItemGetBonus {
				api_type: KcApiQuestClearItemBonusType::AirUnitBase as i64,
				api_count: 0,
				api_item: Some(KcApiQuestClearItemGetBonusItem {
					api_id: Some(reward.api_id),
					api_message: Some(format!(
						"飛行場設営完了！　{name}に「基地航空隊」を展開しました！"
					)),
					api_message_a: Some(format!("{name}に「基地航空隊」を展開中…")),
					..Default::default()
				}),
			})
		}
		Kc3rdQuestRewardCategory::FactoryImprovementUnlock => None,
	};

	Ok(bonus)
}

fn get_quest_rewards(
	codex: &Codex,
	quest_id: i64,
	choices: Option<Vec<i64>>,
) -> Result<KcApiQuestClearItemGet, RewardError> {
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
		.map(|v| convert_kc3rd_quest_reward_to_api(&codex.manifest, v))
		.collect::<Result<Vec<Option<KcApiQuestClearItemGetBonus>>, RewardError>>()?
		.into_iter()
		.flatten()
		.collect();

	Ok(KcApiQuestClearItemGet {
		api_material,
		api_bounus_count: api_bounus.len() as i64,
		api_bounus,
	})
}
