use std::{fs, path::Path};

use emukc_model::prelude::*;
use serde::{Deserialize, Serialize};

use super::error::ParseError;

/// Parse the ship info from the raw string.
///
/// # Arguments
///
/// * `src` - The path to the file to parse, `ships.nedb`.
///
/// # Returns
///
/// A map of ship id to ship info.
pub fn parse(src: impl AsRef<Path>) -> Result<Kc3rdShipBasicMap, ParseError> {
	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct ShipStat {
		evasion: i64,
		evasion_max: i64,
		asw: i64,
		asw_max: i64,
		los: i64,
		los_max: i64,
		luck: i64,
		luck_max: i64,
	}

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct ShipDbEntry {
		id: i64,
		stat: ShipStat,
		class_no: Option<ClassNo>,
		slot: Vec<i64>,
		equip: Equip,
	}

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum ClassNo {
		Integer(i64),
		String(String),
	}

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum Equip {
		String(String),
		UnionArray(Vec<Option<EquipElement>>),
	}

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	#[serde(untagged)]
	pub enum EquipElement {
		EquipClass(EquipClass),
		Integer(i64),
		String(String),
	}

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	pub struct EquipClass {
		pub id: i64,
		pub star: i64,
	}

	let src = src.as_ref();
	trace!("reading {:?}", src);
	let raw = fs::read_to_string(src)?;
	trace!("parsing ship data");
	let mut ships: Vec<Kc3rdShipBasic> = vec![];

	for line in raw.lines() {
		let entry: ShipDbEntry = serde_json::from_str(line)?;
		let mut equip: Vec<Kc3rdShipSlotItem> = vec![];
		if let Equip::UnionArray(elems) = entry.equip {
			for elem in elems {
				match elem {
					Some(EquipElement::EquipClass(equip_class)) => {
						equip.push(Kc3rdShipSlotItem {
							api_id: equip_class.id,
							star: equip_class.star,
						});
					}
					Some(EquipElement::Integer(id)) => {
						equip.push(Kc3rdShipSlotItem {
							api_id: id,
							star: 0,
						});
					}
					_ => {}
				}
			}
		}
		let cnum = match entry.class_no {
			Some(ClassNo::Integer(class_no)) => class_no,
			Some(ClassNo::String(raw_class)) => raw_class
				.chars()
				.filter(char::is_ascii_digit)
				.collect::<String>()
				.parse()
				.unwrap_or(1),
			_ => 1,
		};

		ships.push(Kc3rdShipBasic {
			api_id: entry.id,
			kaih: [entry.stat.evasion, entry.stat.evasion_max],
			tais: [entry.stat.asw, entry.stat.asw_max],
			saku: [entry.stat.los, entry.stat.los_max],
			luck: [entry.stat.luck, entry.stat.luck_max],
			cnum,
			slots: entry.slot,
			equip,
		});
	}
	trace!("{} ships parsed", ships.len());

	let mut map = Kc3rdShipBasicMap::new();
	for ship in ships {
		map.insert(ship.api_id, ship);
	}

	Ok(map)
}
