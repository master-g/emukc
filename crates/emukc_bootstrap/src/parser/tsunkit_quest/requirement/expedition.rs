use emukc_model::prelude::*;

use crate::parser::tsunkit_quest::{Id, Requirements};

impl Requirements {
	pub(super) fn extract_requirements_expedition(&self) -> Vec<Kc3rdQuestCondition> {
		let mut conditions: Vec<Kc3rdQuestCondition> = Vec::new();

		if let Some(res) = self.extract_resource_consumption() {
			conditions.push(res);
		}

		if let Some(expeds) = &self.expeds {
			let expeds: Vec<Kc3rdQuestConditionExpedition> = expeds
				.iter()
				.map(|exped| {
					let times = exped.times;
					let list = exped.id.as_ref().map(|ids| match ids {
						Id::String(id) => vec![id.clone()],
						Id::StringArray(ids) => ids.clone(),
					});
					Kc3rdQuestConditionExpedition {
						list,
						times,
					}
				})
				.collect();
			conditions.push(Kc3rdQuestCondition::Expedition(expeds))
		} else {
			error!("expedition requirement must have key named 'expeds'");
		}

		conditions
	}
}
