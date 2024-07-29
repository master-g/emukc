use crate::{start2::ApiManifest, Kc3rdQuestConditionShip};

use super::Kc3rdQuestDebugJson;

impl Kc3rdQuestDebugJson for Kc3rdQuestConditionShip {
	fn to_json(&self, mst: &ApiManifest) -> serde_json::Value {
		match self {
			Kc3rdQuestConditionShip::Any => serde_json::Value::String("ANY".to_string()),
			Kc3rdQuestConditionShip::Ship(id) => {
				let ship_name =
					mst.find_ship(*id).map(|ship| ship.api_name.clone()).unwrap_or_else(|| {
						error!("Unknown ship ID: {}", id);
						"n/a".to_string()
					});
				serde_json::Value::String(ship_name)
			}
			Kc3rdQuestConditionShip::Ships(ids) => {
				let ships = ids
					.iter()
					.map(|id| {
						mst.find_ship(*id).map(|ship| ship.api_name.clone()).unwrap_or_else(|| {
							error!("Unknown ship ID: {}", id);
							"n/a".to_string()
						})
					})
					.collect::<Vec<String>>();
				serde_json::Value::Array(ships.into_iter().map(serde_json::Value::String).collect())
			}
			Kc3rdQuestConditionShip::ShipType(t) => {
				let type_name =
					mst.find_ship_type(*t).map(|t| t.api_name.clone()).unwrap_or_else(|| {
						error!("Unknown ship type ID: {}", t);
						"n/a".to_string()
					});
				serde_json::Value::String(type_name)
			}
			Kc3rdQuestConditionShip::ShipTypes(ids) => {
				let types = ids
					.iter()
					.map(|id| {
						mst.find_ship_type(*id).map(|t| t.api_name.clone()).unwrap_or_else(|| {
							error!("Unknown ship type ID: {}", id);
							"n/a".to_string()
						})
					})
					.collect::<Vec<String>>();
				serde_json::Value::Array(types.into_iter().map(serde_json::Value::String).collect())
			}
			Kc3rdQuestConditionShip::ShipClass(id) => {
				let class_name =
					mst.find_ship_class(*id).map(|c| c.api_name.clone()).unwrap_or_else(|| {
						error!("Unknown ship class ID: {}", id);
						"n/a".to_string()
					});
				serde_json::Value::String(class_name)
			}
			Kc3rdQuestConditionShip::ShipClasses(ids) => {
				let classes = ids
					.iter()
					.map(|id| {
						mst.find_ship_class(*id).map(|c| c.api_name.clone()).unwrap_or_else(|| {
							error!("Unknown ship class ID: {}", id);
							"n/a".to_string()
						})
					})
					.collect::<Vec<String>>();
				serde_json::Value::Array(
					classes.into_iter().map(serde_json::Value::String).collect(),
				)
			}
			Kc3rdQuestConditionShip::HighSpeed => {
				serde_json::Value::String("HIGH_SPEED".to_string())
			}
			Kc3rdQuestConditionShip::LowSpeed => serde_json::Value::String("LOW_SPEED".to_string()),
			Kc3rdQuestConditionShip::Aviation => serde_json::Value::String("AVIATION".to_string()),
			Kc3rdQuestConditionShip::Carrier => serde_json::Value::String("CARRIER".to_string()),
		}
	}
}
