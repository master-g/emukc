//! Tests for quest reward calculation and validation

#[cfg(test)]
mod tests {
	use emukc_internal::model::codex::Codex;
	use emukc_internal::model::thirdparty::reward::{RewardError, get_quest_rewards};

	fn load_codex() -> Codex {
		Codex::load(std::path::Path::new(".data/codex"), true).unwrap()
	}

	#[test]
	fn test_get_quest_rewards_nonexistent_quest_returns_error() {
		let codex = load_codex();
		let result = get_quest_rewards(&codex, 999999, None);
		assert!(result.is_err());
		let err = result.unwrap_err();
		assert!(matches!(err, RewardError::QuestNotFound(999999)));
	}

	#[test]
	fn test_get_quest_rewards_no_choices_for_non_choice_quest() {
		let codex = load_codex();
		// Find a quest that has no choice rewards
		let quest = codex
			.quest
			.values()
			.find(|q| q.choice_rewards.is_empty())
			.expect("should have at least one quest without choices");

		let result = get_quest_rewards(&codex, quest.api_no, None);
		assert!(result.is_ok());
	}

	#[test]
	fn test_get_quest_rewards_choice_mismatch_returns_error() {
		let codex = load_codex();
		// Find a quest that has choice rewards
		let quest = codex.quest.values().find(|q| !q.choice_rewards.is_empty());

		if let Some(quest) = quest {
			// Pass wrong number of choices
			let wrong_choices = vec![0i64; quest.choice_rewards.len() + 1];
			let result = get_quest_rewards(&codex, quest.api_no, Some(&wrong_choices));
			assert!(result.is_err());
			assert!(matches!(result.unwrap_err(), RewardError::ChoicesLengthMismatch { .. }));
		}
	}

	#[test]
	fn test_get_quest_rewards_valid_choices() {
		let codex = load_codex();
		// Find a quest that has choice rewards
		let quest = codex.quest.values().find(|q| !q.choice_rewards.is_empty());

		if let Some(quest) = quest {
			let choices: Vec<i64> = quest.choice_rewards.iter().map(|_| 0).collect();
			let result = get_quest_rewards(&codex, quest.api_no, Some(&choices));
			assert!(result.is_ok());
			let resp = result.unwrap();
			assert_eq!(resp.api_material[0], quest.reward_fuel);
			assert_eq!(resp.api_material[1], quest.reward_ammo);
			assert_eq!(resp.api_material[2], quest.reward_steel);
			assert_eq!(resp.api_material[3], quest.reward_bauxite);
		}
	}

	#[test]
	fn test_get_quest_rewards_material_amounts_match_manifest() {
		let codex = load_codex();
		// Test a few quests to verify material amounts match
		for quest in codex.quest.values().take(5) {
			let choices = if quest.choice_rewards.is_empty() {
				None
			} else {
				let list: Vec<i64> = quest.choice_rewards.iter().map(|_| 0).collect();
				Some(list)
			};

			let result = get_quest_rewards(&codex, quest.api_no, choices.as_deref());
			if let Ok(resp) = result {
				assert_eq!(
					resp.api_material,
					[
						quest.reward_fuel,
						quest.reward_ammo,
						quest.reward_steel,
						quest.reward_bauxite
					],
					"material mismatch for quest {}",
					quest.api_no
				);
			}
		}
	}
}
