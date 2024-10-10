use crate::{kc2::start2::ApiManifest, thirdparty::Kc3rdQuestConditionShipGroup};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionShipGroup {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let while_list = self.white_list.as_ref().map(|white_list| {
			white_list
				.iter()
				.map(|&id| {
					mst.find_ship(id).map_or_else(
						|| {
							error!("Unknown ship ID: {}", id);
							"n/a".to_string()
						},
						|ship| ship.api_name.clone(),
					)
				})
				.collect::<Vec<String>>()
		});

		serde_json::json!({
			"ship": self.ship.to_json(mst),
			"position": self.position,
			"lv": self.lv,
			"amount": self.amount,
			"white_list": while_list,
		})
	}
}
