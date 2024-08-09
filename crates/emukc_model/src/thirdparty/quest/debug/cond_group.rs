use crate::{kc2::start2::ApiManifest, thirdparty::Kc3rdQuestConditionShipGroup};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionShipGroup {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let while_list = if let Some(white_list) = &self.white_list {
			let name_list = white_list
				.iter()
				.map(|id| {
					let ship = mst.find_ship(*id);
					ship.map(|ship| ship.api_name.clone()).unwrap_or_else(|| {
						error!("Unknown ship ID: {}", id);
						"n/a".to_string()
					})
				})
				.collect::<Vec<String>>();
			Some(name_list)
		} else {
			None
		};

		serde_json::json!({
			"ship": self.ship.to_json(mst),
			"position": self.position,
			"lv": self.lv,
			"amount": self.amount,
			"white_list": while_list,
		})
	}
}
