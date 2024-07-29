mod cond_conversion;
mod cond_exercise;
mod cond_expedition;
mod cond_group;
mod cond_modernization;
mod cond_ship;
mod cond_sortie;
mod cond_useitem;
mod condition;
mod requirement;
mod reward;

use crate::start2::ApiManifest;

use super::Kc3rdQuest;

pub trait Kc3rdQuestDebugJson {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value;
}

impl Kc3rdQuestDebugJson for Kc3rdQuest {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let additional_rewards = self
			.additional_rewards
			.iter()
			.map(|reward| reward.to_json(mst))
			.collect::<Vec<serde_json::Value>>();

		let choice_rewards = self
			.choice_rewards
			.iter()
			.map(|choices| {
				choices
					.choices
					.iter()
					.map(|reward| reward.to_json(mst))
					.collect::<Vec<serde_json::Value>>()
			})
			.collect::<Vec<Vec<serde_json::Value>>>();

		let requirement = match &self.requirements {
			crate::Kc3rdQuestRequirement::And(req) => serde_json::json!({
				"type": "AND",
				"conds": req.iter().map(|cond| cond.to_json(mst)).collect::<Vec<serde_json::Value>>(),
			}),
			crate::Kc3rdQuestRequirement::OneOf(req) => serde_json::json!({
				"type": "ONE_OF",
				"conds": req.iter().map(|cond| cond.to_json(mst)).collect::<Vec<serde_json::Value>>(),
			}),
			crate::Kc3rdQuestRequirement::Sequential(req) => serde_json::json!({
				"type": "SEQUENTIAL",
				"conds": req.iter().map(|cond| cond.to_json(mst)).collect::<Vec<serde_json::Value>>(),
			}),
		};

		serde_json::json!({
			"api_no": self.api_no,
			"wiki_id": self.wiki_id,
			"category": self.category,
			"period": self.period,
			"name": self.name,
			"detail": self.detail,
			"label_type": self.label_type,
			"prerequisite": self.prerequisite,
			"res_reward": vec![self.reward_fuel, self.reward_ammo, self.reward_steel, self.reward_bauxite],
			"additional_rewards": additional_rewards,
			"choice_rewards": choice_rewards,
			"requirement": requirement,
		})
	}
}
