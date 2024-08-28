use emukc_model::prelude::*;

use crate::parser::tsunkit_quest::Requirements;

impl Requirements {
	pub(super) fn extract_requirements_exercise(
		&self,
		mst: &ApiManifest,
	) -> Vec<Kc3rdQuestCondition> {
		let times = self.times.unwrap_or(1);
		let expire_next_day = self.daily.unwrap_or(false);
		let groups = if let Some(comp) = &self.comp {
			let groups = comp.iter().filter_map(|c| c.to_kc3rd_ship_group(mst)).collect();
			Some(groups)
		} else {
			None
		};
		let expect_result = if let Some(result) = &self.result {
			result.to_kc3rd_combat_result()
		} else {
			KcSortieResult::Any
		};
		vec![Kc3rdQuestCondition::Excercise(Kc3rdQuestConditionExcerise {
			times,
			expect_result,
			expire_next_day,
			groups,
		})]
	}
}
