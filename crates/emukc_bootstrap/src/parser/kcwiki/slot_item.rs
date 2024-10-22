use std::{collections::BTreeMap, path::Path};

use emukc_model::prelude::{
	Kc3rdSlotItem, Kc3rdSlotItemAswDamageType, Kc3rdSlotItemImproveBaseConsumption,
	Kc3rdSlotItemImproveItemConsumption, Kc3rdSlotItemImprovePerLevelConsumption,
	Kc3rdSlotItemImproveRequirements, Kc3rdSlotItemImproveSecretary, Kc3rdSlotItemImprovment,
	Kc3rdSlotItemRemodelVariant,
};
use serde::{Deserialize, Serialize};

use crate::parser::error::ParseError;

use super::ParseContext;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrString {
	Bool(bool),
	String(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BoolOrInt {
	Bool(bool),
	Int(i64),
}

impl From<BoolOrInt> for Option<i64> {
	fn from(b: BoolOrInt) -> Self {
		match b {
			BoolOrInt::Bool(_) => None,
			BoolOrInt::Int(i) => Some(i),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrInt {
	String(String),
	Int(i64),
}

impl From<StringOrInt> for i64 {
	fn from(b: StringOrInt) -> Self {
		match b {
			StringOrInt::String(s) => s.parse().unwrap(),
			StringOrInt::Int(i) => i,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AswDamageType {
	#[serde(rename = "DCP")]
	Dcp,
	#[serde(rename = "DCR")]
	Dcr,
}

impl From<AswDamageType> for Kc3rdSlotItemAswDamageType {
	fn from(value: AswDamageType) -> Self {
		match value {
			AswDamageType::Dcp => Self::DepthCargeProjector,
			AswDamageType::Dcr => Self::DepthChargeRack,
		}
	}
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

// pub type EquipBonusMap = std::collections::BTreeMap<String, EquipBonus>;

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
	// we are not there yet
	// #[serde(rename = "_bonus")]
	// pub bonus: Option<EquipBonusMap>,
	// #[serde(rename = "_gun_fit_group")]
	// pub gun_fit_group: Option<String>,
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
			name: value.japanese_name,
			info: value.info,
			craftable: value.buildable,
			stars: value.stars,
			flight_cost: value.flight_cost.and_then(Into::into),
			flight_range: value.flight_range.and_then(Into::into),
			can_attack_installations: value.can_attack_installations.unwrap_or(false),
			asw_damage_type: value.asw_damage_type.map(Into::into),
			improvement: None,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KcwikiSlotitemParsed {
	pub map: BTreeMap<i64, Kc3rdSlotItem>,
	pub wiki_map: BTreeMap<String, KcwikiSlotitem>,
}

/// Parse the `kcwiki_slotitem.json` first-pass for EN name to `mst_id` mapping.
pub(super) fn parse_slotitem_name_mapping(
	src: impl AsRef<Path>,
) -> Result<BTreeMap<String, i64>, ParseError> {
	let src = src.as_ref();
	trace!("parsing kcwiki slotitem for name mapping: {:?}", src);

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct Entry {
		#[serde(rename = "_id")]
		id: i64,

		#[serde(rename = "_name")]
		name: String,
	}

	let map: BTreeMap<String, Entry> = serde_json::from_reader(std::fs::File::open(src)?)?;

	for (k, v) in map.iter() {
		if k != &v.name {
			error!("{} != {}", k, v.name);
		}
	}

	let map = map.into_iter().map(|(k, v)| (k, v.id)).collect();

	Ok(map)
}

fn parse_level_consumption(
	mst_id: i64,
	context: &ParseContext,
	product: &Product,
) -> Result<Kc3rdSlotItemImprovePerLevelConsumption, ParseError> {
	if let Product::Level2Consumption(consumption) = product {
		let mut slot_item_consumptions = vec![];
		let mut use_item_consumptions = vec![];

		match &consumption.equipment {
			ImprovmentEquipConsumption::Bool(true) => {
				error!("`{}` has equipment improvements, but a `true` is unexpected", mst_id);
			}
			ImprovmentEquipConsumption::Map(map) => {
				for (k, v) in map.iter() {
					if k == "true" {
						slot_item_consumptions.push(Kc3rdSlotItemImproveItemConsumption {
							id: mst_id,
							count: *v,
						});
					} else if let Some(id) = context.find_slotitem_id(k) {
						slot_item_consumptions.push(Kc3rdSlotItemImproveItemConsumption {
							id,
							count: *v,
						});
					} else if let Some(id) = context.find_useitem_id(k) {
						use_item_consumptions.push(Kc3rdSlotItemImproveItemConsumption {
							id,
							count: *v,
						});
					} else {
						error!("{} -> `{}` not found", mst_id, k);
					}
				}
			}
			_ => {}
		};

		return Ok(Kc3rdSlotItemImprovePerLevelConsumption {
			dev_mat_min: consumption.development_material,
			dev_mat_max: consumption.development_material_x.clone().into(),
			screw_min: consumption.improvement_material,
			screw_max: consumption.improvement_material_x.clone().into(),
			slot_item_consumption: if slot_item_consumptions.is_empty() {
				None
			} else {
				Some(slot_item_consumptions)
			},
			use_item_consumption: if use_item_consumptions.is_empty() {
				None
			} else {
				Some(use_item_consumptions)
			},
		});
	}

	Err(ParseError::KeyMissing)
}

fn parse_secretary(
	context: &ParseContext,
	product: &Product,
) -> Result<Vec<Kc3rdSlotItemImproveSecretary>, ParseError> {
	let mut result = vec![];

	if let Product::Secretary2WeekInfo(info) = product {
		for (k, week_info) in info.iter() {
			let id = if k == "true" {
				// any ship will do
				0
			} else if let Some(id) = context.find_ship_id(k) {
				id
			} else {
				warn!("ship `{}` not found", k);
				continue;
			};

			result.push(Kc3rdSlotItemImproveSecretary {
				id,
				monday: week_info.monday.unwrap_or(false),
				tuesday: week_info.tuesday.unwrap_or(false),
				wednesday: week_info.wednesday.unwrap_or(false),
				thursday: week_info.thursday.unwrap_or(false),
				friday: week_info.friday.unwrap_or(false),
				saturday: week_info.saturday.unwrap_or(false),
				sunday: week_info.sunday.unwrap_or(false),
			});
		}
	}
	Ok(result)
}

/// Parse the slot item extra info.
///
/// # Arguments
///
/// * `src` - The source directory.
pub(super) fn parse(
	context: &ParseContext,
	src: impl AsRef<Path>,
) -> Result<KcwikiSlotitemParsed, ParseError> {
	let wiki_map = parse_kcwiki_items(src)?;

	let mut map: BTreeMap<i64, Kc3rdSlotItem> = BTreeMap::new();

	for (slot_item_en_name, wiki_item_obj) in wiki_map.iter() {
		if slot_item_en_name != &wiki_item_obj.name {
			error!("{} != {}", slot_item_en_name, wiki_item_obj.name);
		}

		let mut item: Kc3rdSlotItem = wiki_item_obj.clone().into();

		item.improvement = match &wiki_item_obj.improvements {
			ImprovementsUnion::Bool(true) => {
				debug!("`{}` has improvements, but a boolean is not enough", slot_item_en_name);
				None
			}
			ImprovementsUnion::Bool(false) => None,
			ImprovementsUnion::ImprovementsClass(info) => {
				if info.products.is_empty() {
					error!("`{}` has no products", slot_item_en_name);
					None
				} else {
					let base_consumption = {
						let fuel: i64 = Into::<Option<i64>>::into(info.fuel).unwrap_or(0);
						let ammo: i64 = Into::<Option<i64>>::into(info.ammo).unwrap_or(0);
						let steel: i64 = Into::<Option<i64>>::into(info.steel).unwrap_or(0);
						let bauxite: i64 = Into::<Option<i64>>::into(info.bauxite).unwrap_or(0);

						Kc3rdSlotItemImproveBaseConsumption {
							fuel,
							ammo,
							steel,
							bauxite,
						}
					};

					let mut level_consumption_option: Option<Kc3rdSlotItemImproveRequirements> =
						None;
					let mut remodel_variants = vec![];

					for (next_key, info_map) in info.products.iter() {
						if next_key == "false" {
							let mut level_consumption = Kc3rdSlotItemImproveRequirements {
								first_half: None,
								second_half: None,
								remodel: None,
								secretary: vec![],
							};
							for (k, product) in info_map.iter() {
								match k.as_str() {
									"0" => {
										level_consumption.first_half =
											Some(parse_level_consumption(
												wiki_item_obj.id,
												context,
												product,
											)?);
									}
									"6" => {
										level_consumption.second_half =
											Some(parse_level_consumption(
												wiki_item_obj.id,
												context,
												product,
											)?);
									}
									"_ships" => {
										level_consumption.secretary =
											parse_secretary(context, product)?;
									}
									"_stars" => {
										// NOTHING TO DO
									}
									_ => {
										error!(
											"unknown key `{}` found in `{}`s `false`",
											k, slot_item_en_name
										);
									}
								}
							}
							if level_consumption.secretary.is_empty() {
								error!("{}, has no secretary", slot_item_en_name);
							}
							level_consumption_option = Some(level_consumption);
						} else if let Some(slot_item_id) = context.find_slotitem_id(next_key) {
							let mut variant = Kc3rdSlotItemRemodelVariant {
								slot_item_id,
								initial_stars: 0,
								requirements: Kc3rdSlotItemImproveRequirements {
									first_half: None,
									second_half: None,
									remodel: None,
									secretary: vec![],
								},
							};
							for (k, product) in info_map.iter() {
								match k.as_str() {
									"0" => {
										variant.requirements.first_half =
											Some(parse_level_consumption(
												wiki_item_obj.id,
												context,
												product,
											)?);
									}
									"6" => {
										variant.requirements.second_half =
											Some(parse_level_consumption(
												wiki_item_obj.id,
												context,
												product,
											)?);
									}
									"10" => {
										variant.requirements.remodel =
											Some(parse_level_consumption(
												wiki_item_obj.id,
												context,
												product,
											)?);
									}
									"_ships" => {
										variant.requirements.secretary =
											parse_secretary(context, product)?;
									}
									"_stars" => {
										if let Product::Stars(Some(stars)) = product {
											match stars {
												BoolOrInt::Int(i) => {
													variant.initial_stars = *i;
												}
												BoolOrInt::Bool(_) => {
													variant.initial_stars = 0;
												}
											}
										}
									}
									_ => {
										error!(
											"unknown key `{}` found in `{}`s `{}`",
											k, slot_item_en_name, next_key
										);
									}
								}
							}
							remodel_variants.push(variant);
						} else {
							warn!("`{}` -> `{}` not found", slot_item_en_name, next_key);
						}
					}

					Some(Kc3rdSlotItemImprovment {
						base_consumption,
						level_consumption: level_consumption_option,
						remodel_variants: if remodel_variants.is_empty() {
							None
						} else {
							Some(remodel_variants)
						},
					})
				}
			}
		};

		map.insert(wiki_item_obj.id, item);
	}

	Ok(KcwikiSlotitemParsed {
		map,
		wiki_map,
	})
}
