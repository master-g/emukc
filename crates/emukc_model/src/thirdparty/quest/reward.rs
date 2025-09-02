use thiserror::Error;

use crate::{
	codex::Codex,
	kc2::{
		KcApiQuestClearItemBonusType, KcApiQuestClearItemGet, KcApiQuestClearItemGetBonus,
		KcApiQuestClearItemGetBonusItem, KcUseItemType, MaterialCategory,
	},
	prelude::{
		ApiManifest, Kc3rdQuest, Kc3rdQuestReward, Kc3rdQuestRewardCategory,
		extra::{slot_item_conversion_extra, use_item_conversion_extra},
	},
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

	#[error("user choices length mismatch: expected {expected}, got {got}")]
	ChoicesLengthMismatch {
		expected: usize,
		got: usize,
	},
}

/// convert `Kc3rdQuestReward` to `KcApiQuestClearItemGetBonus` for most cases
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

// 646, 648, 651, 652, 1101, 1105, 1112, 1114, 1130
fn get_item_conversion_quest_rewards(
	codex: &Codex,
	quest_manifest: &Kc3rdQuest,
	choices: Option<Vec<i64>>,
) -> Result<KcApiQuestClearItemGet, RewardError> {
	let choices = choices.unwrap_or_default();
	if choices.len() != quest_manifest.choice_rewards.len() {
		warn!(
			"choices length mismatch: expected {}, got {}",
			quest_manifest.choice_rewards.len(),
			choices.len()
		);
		return Err(RewardError::ChoicesLengthMismatch {
			expected: quest_manifest.choice_rewards.len(),
			got: choices.len(),
		});
	}

	let mut api_bounus: Vec<KcApiQuestClearItemGetBonus> = Vec::new();
	for (choice, reward) in choices.iter().zip(quest_manifest.choice_rewards.iter()) {
		let reward = reward.choices.get(*choice as usize);
		if let Some(reward) = reward {
			if let Some(bonus) = convert_kc3rd_quest_reward_to_api(&codex.manifest, reward)? {
				api_bounus.push(bonus);
			}
		} else {
			warn!("invalid choice index: {}", choice);
		}
	}

	quest_manifest.additional_rewards.iter().for_each(|v| {
		if let Ok(Some(mut bonus)) = convert_kc3rd_quest_reward_to_api(&codex.manifest, v) {
			use_item_conversion_extra(codex, quest_manifest.api_no, &mut bonus);
			bonus.api_type = KcApiQuestClearItemBonusType::ModelChange2 as i64;
			api_bounus.push(bonus);
		}
	});

	let result = KcApiQuestClearItemGet {
		api_material: [
			quest_manifest.reward_fuel,
			quest_manifest.reward_ammo,
			quest_manifest.reward_steel,
			quest_manifest.reward_bauxite,
		],
		api_bounus_count: api_bounus.len() as i64,
		api_bounus,
	};

	Ok(result)
}

fn get_model_conversion_quest_rewards(
	codex: &Codex,
	quest_manifest: &Kc3rdQuest,
	choices: Option<Vec<i64>>,
) -> Result<KcApiQuestClearItemGet, RewardError> {
	let choices = choices.unwrap_or_default();
	if choices.len() != quest_manifest.choice_rewards.len() {
		warn!(
			"choices length mismatch: expected {}, got {}",
			quest_manifest.choice_rewards.len(),
			choices.len()
		);
		return Err(RewardError::ChoicesLengthMismatch {
			expected: quest_manifest.choice_rewards.len(),
			got: choices.len(),
		});
	}

	let mut api_bounus: Vec<KcApiQuestClearItemGetBonus> = Vec::new();
	for (choice, reward) in choices.iter().zip(quest_manifest.choice_rewards.iter()) {
		let reward = reward.choices.get(*choice as usize);
		if let Some(reward) = reward {
			if let Some(bonus) = convert_kc3rd_quest_reward_to_api(&codex.manifest, reward)? {
				api_bounus.push(bonus);
			}
		} else {
			warn!("invalid choice index: {}", choice);
		}
	}

	quest_manifest.additional_rewards.iter().for_each(|v| {
		if let Ok(Some(mut bonus)) = convert_kc3rd_quest_reward_to_api(&codex.manifest, v) {
			slot_item_conversion_extra(codex, quest_manifest.api_no, &mut bonus);
			bonus.api_type = KcApiQuestClearItemBonusType::ModelChange as i64;
			api_bounus.push(bonus);
		}
	});

	let result = KcApiQuestClearItemGet {
		api_material: [
			quest_manifest.reward_fuel,
			quest_manifest.reward_ammo,
			quest_manifest.reward_steel,
			quest_manifest.reward_bauxite,
		],
		api_bounus_count: api_bounus.len() as i64,
		api_bounus,
	};

	Ok(result)
}

/// Get request reward for kcs API
///
/// # Arguments
///
/// * `codex` - A reference to the Codex instance
/// * `quest_id` - The ID of the quest
/// * `choices` - Optional user choices, starts from 0
pub fn get_quest_rewards(
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

	if quest_manifest.has_slot_item_consumption() {
		if quest_manifest.has_slot_item_reward() {
			// model conversion quest
			return get_model_conversion_quest_rewards(codex, quest_manifest, choices);
		} else if quest_manifest.has_use_item_reward() {
			// item conversion quest
			return get_item_conversion_quest_rewards(codex, quest_manifest, choices);
		}
	}

	let mut api_bounus: Vec<KcApiQuestClearItemGetBonus> = Vec::new();
	if let Some(user_choices) = choices {
		if user_choices.len() != quest_manifest.choice_rewards.len() {
			warn!(
				"choices length mismatch: expected {}, got {}",
				quest_manifest.choice_rewards.len(),
				user_choices.len()
			);
		} else {
			for (choice, reward) in user_choices.iter().zip(quest_manifest.choice_rewards.iter()) {
				let reward = reward.choices.get(*choice as usize);
				if let Some(reward) = reward {
					if let Some(bonus) = convert_kc3rd_quest_reward_to_api(&codex.manifest, reward)?
					{
						api_bounus.push(bonus);
					}
				} else {
					warn!("invalid choice index: {}", choice);
				}
			}
		}
	}

	let additional_rewards: Vec<KcApiQuestClearItemGetBonus> = quest_manifest
		.additional_rewards
		.iter()
		.map(|v| convert_kc3rd_quest_reward_to_api(&codex.manifest, v))
		.collect::<Result<Vec<Option<KcApiQuestClearItemGetBonus>>, RewardError>>()?
		.into_iter()
		.flatten()
		.collect();

	api_bounus.extend(additional_rewards);

	Ok(KcApiQuestClearItemGet {
		api_material,
		api_bounus_count: api_bounus.len() as i64,
		api_bounus,
	})
}
