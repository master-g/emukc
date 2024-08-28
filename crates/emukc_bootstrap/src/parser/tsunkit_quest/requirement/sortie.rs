use emukc_model::prelude::*;

use crate::parser::tsunkit_quest::Requirements;

impl Requirements {
	pub(super) fn extract_requirements_sortie(
		&self,
		mst: &ApiManifest,
	) -> Vec<Kc3rdQuestCondition> {
		let mut result: Vec<Kc3rdQuestCondition> = Vec::new();
		let fleet = self.extract_fleet(mst);
		if let Some(fleet) = fleet {
			result.push(Kc3rdQuestCondition::Composition(fleet));
		}
		if let Some(sorties) = &self.sortie {
			sorties.iter().for_each(|sortie| {
				let kc3_sortie = sortie.to_kc3rd_sortie();
				result.push(Kc3rdQuestCondition::Sortie(kc3_sortie));
			});
		} else {
			error!("sortie requirement must have a 'sortie' field");
			return vec![];
		}
		result
	}
}
