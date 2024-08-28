use emukc_model::prelude::*;

use crate::parser::tsunkit_quest::Requirements;

impl Requirements {
	pub(super) fn extract_requirements_modernization(
		&self,
		mst: &ApiManifest,
	) -> Vec<Kc3rdQuestCondition> {
		let mut all: Vec<Kc3rdQuestCondition> = Vec::new();

		let times = self.times.unwrap_or(0);
		let target_ship = match &self.class_id {
			Some(class_id) => class_id.to_kc3rd_ship_class(mst),
			None => match &self.family_id {
				Some(family_id) => Some(Kc3rdQuestConditionShip::ShipClass(*family_id)),
				None => {
					error!("modernization requirement must have a class_id or family_id");
					return vec![];
				}
			},
		};

		let Some(consumes) = &self.consume else {
			error!("modernization requirement must have a consume");
			return vec![];
		};

		if consumes.is_empty() || consumes.len() > 1 {
			error!("modernization requirement must have exactly one consume");
			return vec![];
		}

		let consume = &consumes[0];
		let material_ship = match &consume.class_id {
			Some(class_id) => class_id.to_kc3rd_ship_types(mst),
			None => {
				error!("modernization requirement consume must have a class_id");
				return vec![];
			}
		};

		let Some(target_ship) = target_ship else {
			error!("modernization requirement target_ship not found");
			return vec![];
		};

		let Some(material_ship) = material_ship else {
			error!("modernization requirement material_ship not found");
			return vec![];
		};

		all.push(Kc3rdQuestCondition::Modernization(Kc3rdQuestConditionModernization {
			target_ship,
			material_ship,
			batch_size: consume.amount,
			times,
		}));

		if let Some(res) = self.extract_resource_consumption() {
			all.push(res);
		}

		all
	}
}
