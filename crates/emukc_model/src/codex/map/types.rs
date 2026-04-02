use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer as SerdeDeserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, enumn::N)]
pub enum MapResetPolicy {
	#[default]
	Never = 0,
	Monthly = 1,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteOperator {
	#[default]
	Eq,
	Gte,
	Lte,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeedClass {
	Slow,
	#[default]
	Fast,
	FastPlus,
	Fastest,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapCatalog {
	pub maps: BTreeMap<i64, MapDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapDefinition {
	pub map_id: i64,
	pub maparea_id: i64,
	pub mapinfo_no: i64,
	pub name: String,
	pub level: i64,
	pub sally_flag: Vec<i64>,
	pub is_event: bool,
	pub reset_policy: MapResetPolicy,
	pub airbase_count: Option<i64>,
	pub gauge_type: Option<i64>,
	pub gauge_count: Option<i64>,
	pub required_defeat_count: Option<i64>,
	pub max_hp: Option<i64>,
	pub default_variant: String,
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub rank_stage_ids: BTreeMap<i64, String>,
	pub variants: BTreeMap<String, MapVariantDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapVariantDefinition {
	pub variant_key: String,
	pub boss_cell_no: i64,
	pub cells: Vec<MapCellDefinition>,
	#[serde(default)]
	pub routing_rules: BTreeMap<i64, Vec<RouteRule>>,
	pub enemy_fleets: BTreeMap<i64, EnemyFleetDefinition>,
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub ship_drops: BTreeMap<i64, Vec<ShipDropDefinition>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub required_defeat_count: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub clear_to_variant_key: Option<String>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub parse_warnings: Vec<String>,
}

pub type MapStageDefinition = MapVariantDefinition;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapCellDefinition {
	pub cell_no: i64,
	pub color_no: i64,
	pub event_id: i64,
	pub event_kind: i64,
	pub next_cells: Vec<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub master_cell_id: Option<i64>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub distance: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnemyFleetDefinition {
	pub cell_no: i64,
	pub battle_kind: i64,
	pub formations: Vec<i64>,
	pub compositions: Vec<EnemyComposition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnemyComposition {
	pub comp_id: String,
	pub weight: i64,
	pub ship_ids: Vec<i64>,
	#[serde(default)]
	pub formation: Option<i64>,
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub raw_ship_names: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShipDropDefinition {
	pub ship_id: i64,
	pub raw_ship_name: String,
	pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum CompactShipDropDefinition {
	ShipId(i64),
	Detailed {
		ship_id: i64,
		#[serde(default, skip_serializing_if = "Vec::is_empty")]
		tags: Vec<String>,
		#[serde(default, skip_serializing_if = "String::is_empty")]
		raw_ship_name: String,
	},
}

impl Serialize for ShipDropDefinition {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		if self.tags.is_empty() && self.raw_ship_name.is_empty() {
			CompactShipDropDefinition::ShipId(self.ship_id).serialize(serializer)
		} else {
			CompactShipDropDefinition::Detailed {
				ship_id: self.ship_id,
				tags: self.tags.clone(),
				raw_ship_name: self.raw_ship_name.clone(),
			}
			.serialize(serializer)
		}
	}
}

impl<'de> Deserialize<'de> for ShipDropDefinition {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: SerdeDeserializer<'de>,
	{
		let compact = CompactShipDropDefinition::deserialize(deserializer)?;
		Ok(match compact {
			CompactShipDropDefinition::ShipId(ship_id) => Self {
				ship_id,
				raw_ship_name: String::new(),
				tags: Vec::new(),
			},
			CompactShipDropDefinition::Detailed {
				ship_id,
				tags,
				raw_ship_name,
			} => Self {
				ship_id,
				raw_ship_name,
				tags,
			},
		})
	}
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouteRule {
	pub from_cell_no: i64,
	pub to_cell_no: i64,
	pub priority: i64,
	#[serde(default)]
	pub weight: Option<i64>,
	#[serde(default)]
	pub probability_pct: Option<f64>,
	pub predicate: RoutePredicate,
	#[serde(default, skip_serializing_if = "String::is_empty")]
	pub raw_text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum RoutePredicate {
	#[default]
	Always,
	VisitedNode {
		cell_nos: Vec<i64>,
		visited: bool,
	},
	VisitedNodeLabel {
		node_labels: Vec<String>,
		visited: bool,
	},
	FleetSize {
		op: RouteOperator,
		value: i64,
	},
	/// Number of ships carrying at least one equipment of the given slotitem type(s).
	/// This counts ships (not individual equipment items).
	EquipmentCount {
		slotitem_types: Vec<i64>,
		op: RouteOperator,
		value: i64,
	},
	ShipTypeCount {
		ship_types: Vec<i64>,
		op: RouteOperator,
		value: i64,
	},
	FlagshipShipType {
		ship_types: Vec<i64>,
	},
	FlagshipShipId {
		ship_ids: Vec<i64>,
	},
	ContainsShipType {
		ship_types: Vec<i64>,
	},
	ContainsShipId {
		ship_ids: Vec<i64>,
	},
	ContainsShipSet {
		ship_types: Vec<i64>,
		ship_ids: Vec<i64>,
	},
	OnlyShipTypes {
		ship_types: Vec<i64>,
	},
	OnlyShipSet {
		ship_types: Vec<i64>,
		ship_ids: Vec<i64>,
	},
	ShipSetCount {
		ship_types: Vec<i64>,
		ship_ids: Vec<i64>,
		op: RouteOperator,
		value: i64,
	},
	ShipSetSpeedCount {
		ship_types: Vec<i64>,
		ship_ids: Vec<i64>,
		speed_op: RouteOperator,
		speed_class: SpeedClass,
		op: RouteOperator,
		value: i64,
	},
	Speed {
		class: SpeedClass,
	},
	LoS {
		formula: Option<String>,
		op: RouteOperator,
		value: i64,
	},
	DrumCanisterCount {
		op: RouteOperator,
		value: i64,
	},
	And(Vec<RoutePredicate>),
	Or(Vec<RoutePredicate>),
	Not(Box<RoutePredicate>),
	FleetSizeWeightedRandom {
		weights: Vec<FleetSizeWeight>,
	},
	Unknown {
		raw_text: String,
	},
	SourceUnknown {
		raw_text: String,
	},
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetSizeWeight {
	pub fleet_size: i64,
	pub probability_pct: f64,
}

#[cfg(test)]
mod tests {
	use super::ShipDropDefinition;

	#[test]
	fn ship_drop_definition_serializes_compactly() {
		let drops = vec![
			ShipDropDefinition {
				ship_id: 1,
				raw_ship_name: "睦月".to_string(),
				tags: Vec::new(),
			},
			ShipDropDefinition {
				ship_id: 2,
				raw_ship_name: "如月".to_string(),
				tags: vec!["limited".to_string()],
			},
		];

		let json = serde_json::to_value(&drops).unwrap();
		assert_eq!(json[0], serde_json::json!({"ship_id": 1, "raw_ship_name": "睦月"}));
		assert_eq!(
			json[1],
			serde_json::json!({
				"ship_id": 2,
				"tags": ["limited"],
				"raw_ship_name": "如月",
			})
		);
	}

	#[test]
	fn ship_drop_definition_deserializes_compact_and_legacy_forms() {
		let json = serde_json::json!([
			1,
			{
				"ship_id": 2,
				"tags": ["limited"],
			},
			{
				"ship_id": 3,
				"raw_ship_name": "綾波",
				"tags": ["rare"],
			}
		]);

		let drops = serde_json::from_value::<Vec<ShipDropDefinition>>(json).unwrap();
		assert_eq!(
			drops,
			vec![
				ShipDropDefinition {
					ship_id: 1,
					raw_ship_name: String::new(),
					tags: Vec::new(),
				},
				ShipDropDefinition {
					ship_id: 2,
					raw_ship_name: String::new(),
					tags: vec!["limited".to_string()],
				},
				ShipDropDefinition {
					ship_id: 3,
					raw_ship_name: "綾波".to_string(),
					tags: vec!["rare".to_string()],
				},
			]
		);
	}
}
