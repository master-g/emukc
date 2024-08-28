use emukc_model::prelude::*;

use super::{OtherCategory, Rewards};

impl Rewards {
	pub(super) fn to_additional_reward(
		&self,
		mst: &ApiManifest,
		wiki_id: &str,
	) -> Option<Vec<Kc3rdQuestReward>> {
		let mut result: Vec<Kc3rdQuestReward> = Vec::new();
		for other in &self.other {
			if other.choices.is_some() {
				debug!("to_additional_reward, choices is set, ignore");
				continue;
			}

			let category = if let Some(category) = &other.category {
				category
			} else {
				debug!("to_additional_reward, not found: {:?}", other);
				continue;
			};

			let api_id = if let Some(id) = &other.id {
				id.to_owned()
			} else {
				0
			};

			let stars = if let Some(stars) = &other.stars {
				stars.to_owned()
			} else {
				0
			};

			let amount = if let Some(amount) = &other.amount {
				amount.to_owned()
			} else {
				1
			};

			if let Some(reward) =
				Self::extract_reward(mst, wiki_id, category, api_id, stars, amount)
			{
				result.push(reward);
			}
		}

		Some(result)
	}

	pub(super) fn to_choice_rewards(
		&self,
		mst: &ApiManifest,
		wiki_id: &str,
	) -> Option<Vec<Kc3rdQuestChoiceReward>> {
		let mut result: Vec<Kc3rdQuestChoiceReward> = Vec::new();
		for other in &self.other {
			let choices = if let Some(choices) = &other.choices {
				choices
			} else {
				debug!("to_choice_rewards, not found: {:?}", other);
				continue;
			};

			let rewards: Vec<Kc3rdQuestReward> = choices
				.iter()
				.filter_map(|choice| {
					let api_id = choice.id.unwrap_or(0);
					let stars = choice.stars.unwrap_or(0);
					let amount = choice.amount.unwrap_or(0);
					Self::extract_reward(mst, wiki_id, &choice.category, api_id, stars, amount)
				})
				.collect();

			if rewards.is_empty() {
				warn!("to_choice_rewards, no rewards found");
			} else {
				result.push(Kc3rdQuestChoiceReward {
					choices: rewards,
				});
			}
		}
		Some(result)
	}

	fn extract_reward(
		mst: &ApiManifest,
		wiki_id: &str,
		category: &OtherCategory,
		api_id: i64,
		stars: i64,
		amount: i64,
	) -> Option<Kc3rdQuestReward> {
		match category {
			super::OtherCategory::Equipment => {
				match mst.find_slotitem(api_id) {
					Some(mst) => {
						debug!("slot item found: {}, {}", mst.api_id, mst.api_name);
					}
					None => {
						return None;
					}
				};

				Some(Kc3rdQuestReward {
					api_id,
					category: Kc3rdQuestRewardCategory::Slotitem,
					amount,
					stars,
				})
			}
			super::OtherCategory::Feature => match wiki_id {
				"A4" => Some(Kc3rdQuestReward {
					api_id: 2,
					category: Kc3rdQuestRewardCategory::FleetUnlock,
					amount,
					stars,
				}),
				"A14" => Some(Kc3rdQuestReward {
					api_id: 3,
					category: Kc3rdQuestRewardCategory::FleetUnlock,
					amount,
					stars,
				}),
				"A16" => Some(Kc3rdQuestReward {
					api_id: 4,
					category: Kc3rdQuestRewardCategory::FleetUnlock,
					amount,
					stars,
				}),
				"A45" => Some(Kc3rdQuestReward {
					api_id: 0,
					category: Kc3rdQuestRewardCategory::FactoryImprovementUnlock,
					amount,
					stars,
				}),
				"D25" => Some(Kc3rdQuestReward {
					api_id: 0,
					category: Kc3rdQuestRewardCategory::ExpeditionSupplyUnlock,
					amount,
					stars,
				}),
				"F10" => Some(Kc3rdQuestReward {
					api_id: 0,
					category: Kc3rdQuestRewardCategory::LargeShipConstructionUnlock,
					amount,
					stars,
				}),
				"F43" => Some(Kc3rdQuestReward {
					api_id: 6,
					category: Kc3rdQuestRewardCategory::AirbaseUnlock,
					amount,
					stars,
				}),
				"B175" => Some(Kc3rdQuestReward {
					api_id: 7,
					category: Kc3rdQuestRewardCategory::AirbaseUnlock,
					amount,
					stars,
				}),
				_ => {
					error!("unknown feature: {}", wiki_id);
					None
				}
			},
			super::OtherCategory::Furniture => {
				match mst.find_furniture(api_id) {
					Some(mst) => {
						debug!("furniture found: {}, {}", mst.api_id, mst.api_title);
					}
					None => return None,
				};

				Some(Kc3rdQuestReward {
					api_id,
					category: Kc3rdQuestRewardCategory::Furniture,
					amount,
					stars,
				})
			}
			super::OtherCategory::Inventory => {
				match mst.find_useitem(api_id) {
					Some(mst) => {
						debug!("use item found: {}, {}", mst.api_id, mst.api_name);
					}
					None => return None,
				};

				Some(Kc3rdQuestReward {
					api_id,
					category: Kc3rdQuestRewardCategory::UseItem,
					amount,
					stars,
				})
			}
			super::OtherCategory::Senka => Some(Kc3rdQuestReward {
				api_id,
				category: Kc3rdQuestRewardCategory::WarResult,
				amount,
				stars,
			}),
			super::OtherCategory::Ship => {
				match mst.find_ship(api_id) {
					Some(mst) => {
						debug!("ship found: {}, {}", mst.api_id, mst.api_name);
					}
					None => return None,
				};

				Some(Kc3rdQuestReward {
					api_id,
					category: Kc3rdQuestRewardCategory::Ship,
					amount,
					stars,
				})
			}
		}
	}
}
