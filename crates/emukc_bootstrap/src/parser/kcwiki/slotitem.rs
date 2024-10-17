use std::{collections::BTreeMap, path::Path};

use emukc_model::prelude::Kc3rdSlotItem;
use serde::{Deserialize, Serialize};

use crate::parser::error::ParseError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrString {
	Bool(bool),
	String(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrInt {
	Bool(bool),
	Int(i64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrInt {
	String(String),
	Int(i64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AswDamageType {
	#[serde(rename = "DCP")]
	Dcp,
	#[serde(rename = "DCR")]
	Dcr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquipBonus {
	#[serde(rename = "_aa", skip_serializing_if = "Option::is_none")]
	pub aa: Option<i64>,

	#[serde(rename = "_evasion", skip_serializing_if = "Option::is_none")]
	pub evasion: Option<i64>,

	#[serde(rename = "_firepower", skip_serializing_if = "Option::is_none")]
	pub firepower: Option<i64>,

	#[serde(rename = "_torpedo", skip_serializing_if = "Option::is_none")]
	pub torpedo: Option<i64>,
}

pub type EquipBonusMap = std::collections::BTreeMap<String, EquipBonus>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImprovementsUnion {
	Bool(bool),
	ImprovementsClass(ImprovementsClass),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct WeekInfo {
	friday: Option<bool>,
	monday: Option<bool>,
	saturday: Option<bool>,
	sunday: Option<bool>,
	thursday: Option<bool>,
	tuesday: Option<bool>,
	wednesday: Option<bool>,
}

pub type Secretary2WeekInfo = BTreeMap<String, WeekInfo>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ImprovmentEquipConsumption {
	Bool(bool),
	Map(BTreeMap<String, i64>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImprovmentExtraConsumption {
	#[serde(rename = "_development_material")]
	pub development_material: i64,
	#[serde(rename = "_development_material_x")]
	pub development_material_x: StringOrInt,
	#[serde(rename = "_equipment")]
	pub equipment: ImprovmentEquipConsumption,
	#[serde(rename = "_improvement_material")]
	pub improvement_material: i64,
	#[serde(rename = "_improvement_material_x")]
	pub improvement_material_x: StringOrInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Product {
	Level2Consumption(ImprovmentExtraConsumption),
	Secretary2WeekInfo(Secretary2WeekInfo),
	Stars(Option<BoolOrInt>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImprovementsClass {
	#[serde(rename = "_ammo")]
	pub ammo: BoolOrInt,
	#[serde(rename = "_bauxite")]
	pub bauxite: BoolOrInt,
	#[serde(rename = "_fuel")]
	pub fuel: BoolOrInt,
	#[serde(rename = "_products")]
	pub products: BTreeMap<String, BTreeMap<String, Product>>,
	#[serde(rename = "_steel")]
	pub steel: BoolOrInt,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KcwikiSlotitem {
	#[serde(rename = "_buildable")]
	pub buildable: bool,
	#[serde(rename = "_id")]
	pub id: i64,
	#[serde(rename = "_improvements")]
	pub improvements: ImprovementsUnion,
	#[serde(rename = "_info")]
	pub info: String,
	#[serde(rename = "_japanese_name")]
	pub japanese_name: String,
	#[serde(rename = "_name")]
	pub name: String,
	#[serde(rename = "_special")]
	pub special: BoolOrString,
	#[serde(rename = "_bonus")]
	pub bonus: Option<EquipBonusMap>,
	#[serde(rename = "_gun_fit_group")]
	pub gun_fit_group: Option<String>,
	#[serde(rename = "_flight_cost")]
	pub flight_cost: Option<BoolOrInt>,
	#[serde(rename = "_flight_range")]
	pub flight_range: Option<BoolOrInt>,
	#[serde(rename = "_stars")]
	pub stars: Option<i64>,
	#[serde(rename = "_can_attack_installations")]
	pub can_attack_installations: Option<bool>,
	#[serde(rename = "_asw_damage_type")]
	pub asw_damage_type: Option<AswDamageType>,
}

fn parse_kcwiki_items(
	src: impl AsRef<Path>,
) -> Result<BTreeMap<String, KcwikiSlotitem>, ParseError> {
	let src = src.as_ref();
	trace!("parsing kcwiki slotitem extra info: {:?}", src);

	let map: BTreeMap<String, KcwikiSlotitem> = serde_json::from_reader(std::fs::File::open(src)?)?;

	Ok(map)
}

impl From<KcwikiSlotitem> for Kc3rdSlotItem {
	fn from(value: KcwikiSlotitem) -> Self {
		Self {
			api_id: value.id,
			name: value.japanese_name.to_string(),
			info: value.info.to_string(),
			buildable: value.buildable,
		}
	}
}

/// Parse the slot item extra info.
///
/// # Arguments
///
/// * `src` - The source directory.
pub fn parse(src: impl AsRef<Path>) -> Result<BTreeMap<i64, Kc3rdSlotItem>, ParseError> {
	let wiki_map = parse_kcwiki_items(src)?;
	println!("{}", wiki_map.len());

	for (k, v) in wiki_map.iter() {
		if k != &v.name {
			println!("{} != {}", k, v.name);
		}
	}

	let map: BTreeMap<i64, Kc3rdSlotItem> =
		wiki_map.into_values().map(|v| (v.id, v.into())).collect();

	Ok(map)
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;

	use crate::parser::kcwiki::slotitem::{
		AswDamageType, ImprovmentEquipConsumption, ImprovmentExtraConsumption, Product,
		StringOrInt, WeekInfo,
	};

	#[test]
	fn test_parse() {
		let src = std::path::Path::new("../../.data/temp/kcwiki_slotitem.json");
		let map = super::parse(src).unwrap();
		println!("{:?}", map);
	}

	#[test]
	fn test_dcr() {
		let a = AswDamageType::Dcr;
		let v = vec![a];
		let j = serde_json::to_string(&v).unwrap();
		println!("{}", j);
	}

	#[test]
	fn test_r_product() {
		let mut map: BTreeMap<String, BTreeMap<String, Product>> = BTreeMap::new();

		let product: Product = Product::Stars(Some(super::BoolOrInt::Bool(false)));

		let mut product_map: BTreeMap<String, Product> = BTreeMap::new();

		product_map.insert("_stars".to_string(), product);

		let level_0_consumption = ImprovmentExtraConsumption {
			development_material: 6,
			development_material_x: StringOrInt::String("6".to_string()),
			equipment: ImprovmentEquipConsumption::Bool(false),
			improvement_material: 3,
			improvement_material_x: StringOrInt::Int(4),
		};

		let level_6_consumption = ImprovmentExtraConsumption {
			development_material: 5,
			development_material_x: StringOrInt::Int(8),
			equipment: ImprovmentEquipConsumption::Map({
				let mut m = BTreeMap::new();
				m.insert("10cm Twin High-angle Gun Mount".to_string(), 2);
				m
			}),
			improvement_material: 4,
			improvement_material_x: StringOrInt::Int(7),
		};

		product_map.insert("0".to_string(), Product::Level2Consumption(level_0_consumption));
		product_map.insert("6".to_string(), Product::Level2Consumption(level_6_consumption));
		product_map.insert(
			"_ships".to_string(),
			Product::Secretary2WeekInfo({
				let mut m = BTreeMap::new();
				m.insert(
					"Akizuki/".to_string(),
					WeekInfo {
						friday: Some(false),
						monday: Some(true),
						saturday: Some(false),
						sunday: Some(false),
						thursday: Some(true),
						tuesday: Some(true),
						wednesday: Some(true),
					},
				);
				m
			}),
		);

		map.insert("false".to_string(), product_map);

		let j = serde_json::to_string_pretty(&map).unwrap();
		println!("{}", j);
	}

	#[test]
	fn test_product() {
		let raw = r#"
{
        "F6F-3": {
          "0": {
            "_development_material": 3,
            "_development_material_x": "6",
            "_equipment": {
              "Type 0 Fighter Model 21": 1
            },
            "_improvement_material": 3,
            "_improvement_material_x": "4"
          },
          "6": {
            "_development_material": 4,
            "_development_material_x": 8,
            "_equipment": {
              "Type 0 Fighter Model 32": 1
            },
            "_improvement_material": 3,
            "_improvement_material_x": 6
          },
          "10": {
            "_development_material": 8,
            "_development_material_x": 16,
            "_equipment": {
              "Type 0 Fighter Model 52": 2
            },
            "_improvement_material": 6,
            "_improvement_material_x": 9
          },
          "_ships": {
            "Saratoga/": {
              "Friday": false,
              "Monday": true,
              "Saturday": false,
              "Sunday": false,
              "Thursday": false,
              "Tuesday": true,
              "Wednesday": false
            },
            "Saratoga/Kai": {
              "Friday": false,
              "Monday": true,
              "Saturday": false,
              "Sunday": false,
              "Thursday": false,
              "Tuesday": true,
              "Wednesday": true
            },
            "Saratoga/Mk.II": {
              "Friday": false,
              "Monday": true,
              "Saturday": false,
              "Sunday": false,
              "Thursday": true,
              "Tuesday": false,
              "Wednesday": false
            }
          }
        }
      }
        "#;

		let m = serde_json::from_str::<BTreeMap<String, BTreeMap<String, Product>>>(raw).unwrap();
		println!("{:?}", m);
	}
}
