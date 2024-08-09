use crate::{
	kc2::start2::ApiManifest,
	thirdparty::{Kc3rdQuestReward, Kc3rdQuestRewardCategory},
};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestReward {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		let debug_name = match self.category {
			Kc3rdQuestRewardCategory::Material => mst
				.find_useitem(self.api_id)
				.map(|item| item.api_name.clone())
				.unwrap_or("n/a".to_owned()),
			Kc3rdQuestRewardCategory::Slotitem => mst
				.find_slotitem(self.api_id)
				.map(|item| item.api_name.clone())
				.unwrap_or("n/a".to_owned()),
			Kc3rdQuestRewardCategory::Ship => mst
				.find_ship(self.api_id)
				.map(|ship| ship.api_name.clone())
				.unwrap_or("n/a".to_owned()),
			Kc3rdQuestRewardCategory::Furniture => mst
				.find_furniture(self.api_id)
				.map(|furniture| furniture.api_title.clone())
				.unwrap_or("n/a".to_owned()),
			Kc3rdQuestRewardCategory::UseItem => mst
				.find_useitem(self.api_id)
				.map(|item| item.api_name.clone())
				.unwrap_or("n/a".to_owned()),
			Kc3rdQuestRewardCategory::FleetUnlock => {
				format!("unlock fleet {}", self.api_id)
			}
			Kc3rdQuestRewardCategory::LargeShipConstructionUnlock => {
				"unlock large ship construction".to_owned()
			}
			Kc3rdQuestRewardCategory::FactoryImprovementUnlock => {
				"unlock factory improvement".to_owned()
			}
			Kc3rdQuestRewardCategory::WarResult => {
				format!("war result {}", self.amount)
			}
			Kc3rdQuestRewardCategory::ExpeditionSupplyUnlock => {
				"unlock expedition supply".to_owned()
			}
			Kc3rdQuestRewardCategory::AirbaseUnlock => {
				format!("unlock airbase {}", self.api_id)
			}
		};

		serde_json::json!({
			"category": self.category,
			"api_id": self.api_id,
			"amount": self.amount,
			"stars": self.stars,
			"debug_name": debug_name,
		})
	}
}
