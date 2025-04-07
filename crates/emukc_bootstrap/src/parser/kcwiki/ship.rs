use emukc_model::prelude::{Kc3rdShip, Kc3rdShipRemodelRequirement, Kc3rdShipSlotInfo};
use serde::{Deserialize, Serialize};
use std::vec;
use std::{collections::BTreeMap, path::Path};

use super::ParseContext;
use super::types::{BoolOrInt, BoolOrString};
use crate::parser::error::ParseError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KcwikiShip {
	#[serde(rename = "_aa")]
	aa: i64,
	#[serde(rename = "_aa_max")]
	aa_max: BoolOrInt,
	#[serde(rename = "_aa_mod")]
	aa_mod: BoolOrInt,
	#[serde(rename = "_ammo")]
	ammo: i64,
	#[serde(rename = "_api_id")]
	api_id: i64,
	#[serde(rename = "_armor")]
	armor: i64,
	#[serde(rename = "_armor_max")]
	armor_max: i64,
	#[serde(rename = "_armor_mod")]
	armor_mod: BoolOrInt,
	#[serde(rename = "_asw")]
	asw: Option<i64>,
	#[serde(rename = "_asw_max")]
	asw_max: Option<BoolOrInt>,
	#[serde(rename = "_buildable")]
	buildable: Option<bool>,
	#[serde(rename = "_buildable_lsc")]
	buildable_lsc: Option<bool>,
	#[serde(rename = "_class_number")]
	class_number: i64,
	#[serde(rename = "_equipment")]
	equipment: Vec<Equipment>,
	#[serde(rename = "_evasion")]
	evasion: Option<i64>,
	#[serde(rename = "_evasion_max")]
	evasion_max: Option<i64>,
	#[serde(rename = "_firepower")]
	firepower: i64,
	#[serde(rename = "_firepower_max")]
	firepower_max: i64,
	#[serde(rename = "_firepower_mod")]
	firepower_mod: BoolOrInt,
	#[serde(rename = "_fuel")]
	fuel: i64,
	#[serde(rename = "_id")]
	id: i64,
	#[serde(rename = "_japanese_name")]
	japanese_name: String,
	#[serde(rename = "_los")]
	los: Option<i64>,
	#[serde(rename = "_los_max")]
	los_max: Option<i64>,
	#[serde(rename = "_luck")]
	luck: i64,
	#[serde(rename = "_luck_max")]
	luck_max: i64,
	#[serde(rename = "_luck_mod")]
	luck_mod: LuckMod,
	#[serde(rename = "_range")]
	range: i64,
	#[serde(rename = "_rarity")]
	rarity: i64,
	#[serde(rename = "_remodel_from")]
	remodel_from: Option<BoolOrString>,
	#[serde(rename = "_remodel_level")]
	remodel_level: BoolOrInt,
	#[serde(rename = "_remodel_to")]
	remodel_to: BoolOrString,
	#[serde(rename = "_torpedo")]
	torpedo: i64,
	#[serde(rename = "_torpedo_max")]
	torpedo_max: BoolOrInt,
	#[serde(rename = "_torpedo_mod")]
	torpedo_mod: BoolOrInt,
	#[serde(rename = "_full_name")]
	full_name: String,
	#[serde(rename = "_remodel_ammo")]
	remodel_ammo: Option<i64>,
	#[serde(rename = "_remodel_blueprint")]
	remodel_blueprint: Option<BoolOrInt>,
	#[serde(rename = "_remodel_catapult")]
	remodel_catapult: Option<BoolOrInt>,
	#[serde(rename = "_remodel_development_material")]
	remodel_development_material: Option<BoolOrInt>,
	#[serde(rename = "_remodel_steel")]
	remodel_steel: Option<i64>,
	#[serde(rename = "_remodel_airmat")]
	remodel_airmat: Option<BoolOrInt>,
	#[serde(rename = "_remodel_report")]
	remodel_report: Option<BoolOrInt>,
	#[serde(rename = "_remodel_construction_material")]
	remodel_construction_material: Option<BoolOrInt>,
	#[serde(rename = "_remodel_to_ammo")]
	remodel_to_ammo: Option<i64>,
	#[serde(rename = "_remodel_to_blueprint")]
	remodel_to_blueprint: Option<bool>,
	#[serde(rename = "_remodel_to_catapult")]
	remodel_to_catapult: Option<bool>,
	#[serde(rename = "_remodel_to_construction_material")]
	remodel_to_construction_material: Option<i64>,
	#[serde(rename = "_remodel_to_development_material")]
	remodel_to_development_material: Option<BoolOrInt>,
	#[serde(rename = "_remodel_to_level")]
	remodel_to_level: Option<i64>,
	#[serde(rename = "_remodel_to_steel")]
	remodel_to_steel: Option<i64>,
	#[serde(rename = "_back")]
	back: Option<i64>,
	#[serde(rename = "_remodel_to_report")]
	remodel_to_report: Option<bool>,
	#[serde(rename = "_remodel_armament")]
	remodel_armament: Option<BoolOrInt>,
	#[serde(rename = "_remodel_screw")]
	remodel_screw: Option<i64>,
	#[serde(rename = "_remodel_gunmat")]
	remodel_gunmat: Option<BoolOrInt>,
	#[serde(rename = "_reversible")]
	reversible: Option<bool>,
	#[serde(rename = "_remodel_overseas")]
	remodel_overseas: Option<BoolOrInt>,
	#[serde(rename = "_remodel_from_fixme")]
	remodel_from_fixme: Option<String>,
	#[serde(rename = "_remodel_boiler")]
	remodel_boiler: Option<i64>,
	// we are not there yet
	// #[serde(rename = "_gun_fit_properties")]
	// gun_fit_properties: Option<GunFitProperties>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Equipment {
	equipment: Option<BoolOrString>,
	size: i64,
	stars: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LuckMod {
	Bool(bool),
	Double(f64),
}

/// Parse the `kcwiki_slotitem.json` first-pass for EN name to `mst_id` mapping.
pub(super) fn parse_ship_name_mapping(
	src: impl AsRef<Path>,
) -> Result<BTreeMap<String, i64>, ParseError> {
	let src = src.as_ref();
	trace!("parsing kcwiki ship for name mapping: {:?}", src);

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct Entry {
		#[serde(rename = "_api_id")]
		id: i64,

		#[serde(rename = "_full_name")]
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

pub(super) fn parse(
	context: &ParseContext,
	src: impl AsRef<Path>,
) -> Result<BTreeMap<i64, Kc3rdShip>, ParseError> {
	let src = src.as_ref();
	trace!("parsing kcwiki ship: {:?}", src);

	let map: BTreeMap<String, KcwikiShip> = serde_json::from_reader(std::fs::File::open(src)?)?;

	let mut result = BTreeMap::new();

	for (ship_en_name, wiki_ship) in map.iter() {
		if ship_en_name != &wiki_ship.full_name {
			error!("`{}` != `{}`", ship_en_name, wiki_ship.full_name);
		}

		let evasion_max: i64 = Into::<Option<i64>>::into(wiki_ship.evasion_max).unwrap_or(0);
		let aws_max: i64 = if let Some(aws_max) = wiki_ship.asw_max {
			Into::<Option<i64>>::into(aws_max).unwrap_or(0)
		} else {
			0
		};

		let api_id = wiki_ship.api_id;

		let mut slots = vec![];

		for equipment in wiki_ship.equipment.iter() {
			let item_id = match &equipment.equipment {
				Some(BoolOrString::String(v)) => {
					if let Some(id) = context.find_slotitem_id(v) {
						id
					} else {
						error!("slot item not found: {}", v);
						0
					}
				}
				None | Some(BoolOrString::Bool(_)) => 0,
			};

			slots.push(Kc3rdShipSlotInfo {
				onslot: equipment.size,
				item_id,
				stars: equipment.stars.unwrap_or(0),
			});
		}

		let remodel = if wiki_ship.remodel_from.is_some() {
			let level: i64 = wiki_ship.remodel_level.into();
			if level == 0 {
				None
			} else {
				Some(Kc3rdShipRemodelRequirement {
					level,
					ammo: wiki_ship.remodel_ammo.unwrap_or(0),
					steel: wiki_ship.remodel_steel.unwrap_or(0),
					blueprint: wiki_ship.remodel_blueprint.map_or(0, Into::into),
					catapult: wiki_ship.remodel_catapult.map_or(0, Into::into),
					report: wiki_ship.remodel_report.map_or(0, Into::into),
					devmat: wiki_ship.remodel_development_material.map_or(0, Into::into),
					torch: wiki_ship.remodel_construction_material.map_or(0, Into::into),
					aviation: wiki_ship.remodel_airmat.map_or(0, Into::into),
					artillery: wiki_ship.remodel_gunmat.map_or(0, Into::into),
					armament: wiki_ship.remodel_armament.map_or(0, Into::into),
					boiler: wiki_ship.remodel_boiler.map_or(0, Into::into),
					overseas: wiki_ship.remodel_overseas.map_or(0, Into::into),
				})
			}
		} else {
			None
		};

		let (remodel_back_to, remodel_back_requirement) = if wiki_ship.reversible.unwrap_or(false) {
			let remodel_to = match &wiki_ship.remodel_to {
				BoolOrString::Bool(_) => 0,
				BoolOrString::String(v) => {
					if let Some(id) = context.find_ship_id(v) {
						id
					} else {
						error!("ship not found: {}", v);
						0
					}
				}
			};
			(
				Some(remodel_to),
				Some(Kc3rdShipRemodelRequirement {
					level: wiki_ship.remodel_to_level.unwrap_or(0),
					ammo: wiki_ship.remodel_to_ammo.unwrap_or(0),
					steel: wiki_ship.remodel_to_steel.unwrap_or(0),
					blueprint: wiki_ship.remodel_to_blueprint.map_or(0, Into::into),
					catapult: wiki_ship.remodel_to_catapult.map_or(0, Into::into),
					report: wiki_ship.remodel_to_report.map_or(0, Into::into),
					devmat: wiki_ship.remodel_to_development_material.map_or(0, Into::into),
					torch: wiki_ship.remodel_to_construction_material.map_or(0, Into::into),
					aviation: 0,
					artillery: 0,
					armament: 0,
					boiler: 0,
					overseas: 0,
				}),
			)
		} else {
			(None, None)
		};

		result.insert(
			api_id,
			Kc3rdShip {
				api_id,
				kaih: [wiki_ship.evasion.unwrap_or(0), evasion_max],
				tais: [wiki_ship.asw.unwrap_or(0), aws_max],
				saku: [wiki_ship.los.unwrap_or(0), wiki_ship.los_max.unwrap_or(0)],
				luck: [wiki_ship.luck, wiki_ship.luck_max],
				luck_bonus: if let LuckMod::Double(v) = wiki_ship.luck_mod {
					v
				} else {
					0.0
				},
				armor_bonus: if let BoolOrInt::Int(v) = wiki_ship.armor_mod {
					v
				} else {
					0
				},
				cnum: wiki_ship.class_number,
				buildable: wiki_ship.buildable.unwrap_or(false),
				buildable_lsc: wiki_ship.buildable_lsc.unwrap_or(false),
				slots,
				remodel,
				remodel_back_to,
				remodel_back_requirement,
			},
		);
	}

	Ok(result)
}
