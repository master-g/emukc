use std::{collections::BTreeMap, path::Path};

use emukc_model::prelude::{
	ApiManifest, ApiMstShip, ApiMstSlotitem, Kc3rdEnemyShip, Kc3rdEnemyShipMap,
	Kc3rdEnemyShipSlotInfo,
};
use serde::Deserialize;

use crate::parser::error::ParseError;

use super::{
	ParseContext,
	types::{BoolOrInt, BoolOrString},
};

#[derive(Debug, Default)]
pub(super) struct EnemyParsed {
	pub ship_map: Kc3rdEnemyShipMap,
	pub manifest_ships: Vec<ApiMstShip>,
	pub manifest_slotitems: Vec<ApiMstSlotitem>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct KcwikiEnemyEquipment {
	#[serde(rename = "_id")]
	id: i64,

	#[serde(rename = "_japanese_name")]
	japanese_name: String,

	#[serde(rename = "_name")]
	name: String,

	#[serde(rename = "_type")]
	item_type: i64,

	#[serde(rename = "_firepower")]
	firepower: Option<BoolOrInt>,

	#[serde(rename = "_torpedo")]
	torpedo: Option<BoolOrInt>,

	#[serde(rename = "_aa")]
	aa: Option<BoolOrInt>,

	#[serde(rename = "_armor")]
	armor: Option<BoolOrInt>,

	#[serde(rename = "_los")]
	los: Option<BoolOrInt>,

	#[serde(rename = "_asw")]
	asw: Option<BoolOrInt>,

	#[serde(rename = "_range")]
	range: Option<BoolOrInt>,

	#[serde(rename = "_evasion")]
	evasion: Option<BoolOrInt>,

	#[serde(rename = "_bombing")]
	bombing: Option<BoolOrInt>,

	#[serde(rename = "_speed")]
	speed: Option<BoolOrInt>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct KcwikiEnemyEquipmentSlot {
	equipment: BoolOrString,
	size: i64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct KcwikiEnemyShip {
	#[serde(rename = "_api_id")]
	api_id: i64,

	#[serde(rename = "_japanese_name")]
	japanese_name: String,

	#[serde(rename = "_reading", default, deserialize_with = "nullable_string")]
	reading: String,

	#[serde(rename = "_type")]
	ship_type: i64,

	#[serde(rename = "_hp", default)]
	hp: Option<BoolOrInt>,

	#[serde(rename = "_firepower", default)]
	firepower: Option<BoolOrInt>,

	#[serde(rename = "_torpedo", default)]
	torpedo: Option<BoolOrInt>,

	#[serde(rename = "_aa", default)]
	aa: Option<BoolOrInt>,

	#[serde(rename = "_armor", default)]
	armor: Option<BoolOrInt>,

	#[serde(rename = "_evasion", default)]
	evasion: Option<BoolOrInt>,

	#[serde(rename = "_asw", default)]
	asw: Option<BoolOrInt>,

	#[serde(rename = "_los", default)]
	los: Option<BoolOrInt>,

	#[serde(rename = "_luck", default)]
	luck: Option<BoolOrInt>,

	#[serde(rename = "_speed", default)]
	speed: Option<BoolOrInt>,

	#[serde(rename = "_range", default)]
	range: Option<BoolOrInt>,

	#[serde(rename = "_rarity", default)]
	rarity: Option<BoolOrInt>,

	#[serde(rename = "_back", default)]
	back: Option<BoolOrInt>,

	#[serde(rename = "_equipment", default)]
	equipment: Vec<KcwikiEnemyEquipmentSlot>,
}

fn nullable_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
	D: serde::Deserializer<'de>,
{
	Option::<String>::deserialize(deserializer).map(|opt| opt.unwrap_or_default())
}

fn normalize_opt_i64(value: Option<BoolOrInt>) -> i64 {
	value.map(Into::into).unwrap_or_default()
}

fn load_json_map<T>(path: impl AsRef<Path>) -> Result<BTreeMap<String, T>, ParseError>
where
	T: serde::de::DeserializeOwned,
{
	let path = path.as_ref();
	let file = std::fs::File::open(path).map_err(|source| ParseError::io_at(path, source))?;
	serde_json::from_reader(file).map_err(|source| ParseError::json_at(path, source))
}

fn insert_slotitem_alias(context: &mut ParseContext, alias: &str, id: i64) {
	if alias.is_empty() {
		return;
	}
	context.slotitem_name_map.entry(alias.to_string()).or_insert(id);
}

fn parse_enemy_equipment(
	context: &mut ParseContext,
	manifest: &ApiManifest,
	path: impl AsRef<Path>,
) -> Result<Vec<ApiMstSlotitem>, ParseError> {
	let raw = load_json_map::<KcwikiEnemyEquipment>(path)?;
	let mut missing = Vec::new();
	for (key, equipment) in raw {
		let alias_id = manifest
			.find_slotitem(equipment.id)
			.or_else(|| manifest.find_slotitem_by_name(&equipment.japanese_name))
			.map(|slotitem| slotitem.api_id)
			.unwrap_or(equipment.id);
		insert_slotitem_alias(context, &key, alias_id);
		insert_slotitem_alias(context, &equipment.name, alias_id);
		insert_slotitem_alias(context, &equipment.japanese_name, alias_id);

		if manifest.find_slotitem(equipment.id).is_some() || alias_id != equipment.id {
			continue;
		}

		missing.push(ApiMstSlotitem {
			api_id: equipment.id,
			api_name: equipment.japanese_name,
			api_type: [0, 0, equipment.item_type, 0, 0],
			api_houg: normalize_opt_i64(equipment.firepower),
			api_raig: normalize_opt_i64(equipment.torpedo),
			api_tyku: normalize_opt_i64(equipment.aa),
			api_souk: normalize_opt_i64(equipment.armor),
			api_saku: normalize_opt_i64(equipment.los),
			api_tais: normalize_opt_i64(equipment.asw),
			api_houk: normalize_opt_i64(equipment.evasion),
			api_baku: normalize_opt_i64(equipment.bombing),
			api_soku: normalize_opt_i64(equipment.speed),
			api_leng: normalize_opt_i64(equipment.range),
			api_sortno: equipment.id,
			api_broken: [0; 4],
			api_usebull: "0".to_string(),
			..ApiMstSlotitem::default()
		});
	}
	Ok(missing)
}

fn enemy_slot_item_id(
	context: &ParseContext,
	ship: &KcwikiEnemyShip,
	slot: &KcwikiEnemyEquipmentSlot,
) -> Result<i64, ParseError> {
	match &slot.equipment {
		BoolOrString::Bool(false) => Ok(-1),
		BoolOrString::Bool(true) => Err(ParseError::Generic(format!(
			"unexpected boolean enemy equipment flag for ship {} ({})",
			ship.api_id, ship.japanese_name
		))),
		BoolOrString::String(name) => context.find_slotitem_id(name).ok_or_else(|| {
			ParseError::KeyMissing(format!(
				"enemy equipment `{name}` for ship {} ({})",
				ship.api_id, ship.japanese_name
			))
		}),
	}
}

fn build_enemy_manifest_ship(enemy: &Kc3rdEnemyShip, existing: Option<&ApiMstShip>) -> ApiMstShip {
	let mut ship = existing.cloned().unwrap_or_default();
	ship.api_id = enemy.api_id;
	ship.api_name = enemy.name.clone();
	ship.api_yomi = enemy.yomi.clone();
	ship.api_stype = enemy.stype;
	if ship.api_ctype == 0 {
		ship.api_ctype = enemy.ctype;
	}
	ship.api_soku = enemy.speed;
	ship.api_sort_id = if ship.api_sort_id > 0 {
		ship.api_sort_id
	} else {
		enemy.api_id
	};
	ship.api_sortno = ship.api_sortno.or(Some(enemy.api_id));
	ship.api_backs = Some(enemy.backs);
	ship.api_slot_num = enemy.slot_num;
	ship.api_leng = Some(enemy.range);
	ship.api_taik = Some([enemy.hp, enemy.hp]);
	ship.api_houg = Some([enemy.firepower, enemy.firepower]);
	ship.api_raig = Some([enemy.torpedo, enemy.torpedo]);
	ship.api_tyku = Some([enemy.aa, enemy.aa]);
	ship.api_souk = Some([enemy.armor, enemy.armor]);
	ship.api_luck = Some([enemy.luck, enemy.luck]);
	ship.api_tais = Some([enemy.asw]);
	ship.api_maxeq = Some(enemy.maxeq);
	ship.api_fuel_max.get_or_insert(0);
	ship.api_bull_max.get_or_insert(0);
	ship
}

fn parse_enemy_ships(
	context: &ParseContext,
	manifest: &ApiManifest,
	path: impl AsRef<Path>,
) -> Result<Kc3rdEnemyShipMap, ParseError> {
	let raw = load_json_map::<KcwikiEnemyShip>(path)?;
	let mut ships = Kc3rdEnemyShipMap::new();

	for (_, ship) in raw {
		let mst = manifest.find_ship(ship.api_id);
		let mut maxeq = [0; 5];
		let mut slots = Vec::new();
		for (idx, slot) in ship.equipment.iter().take(5).enumerate() {
			maxeq[idx] = slot.size.max(0);
			let item_id = enemy_slot_item_id(context, &ship, slot)?;
			if item_id > 0 {
				slots.push(Kc3rdEnemyShipSlotInfo {
					item_id,
					onslot: slot.size.max(0),
				});
			}
		}

		let rarity = normalize_opt_i64(ship.rarity);
		let back = normalize_opt_i64(ship.back);
		let backs = if back >= 0 {
			back
		} else {
			mst.and_then(|entry| entry.api_backs).unwrap_or(rarity.max(0))
		};

		ships.insert(
			ship.api_id,
			Kc3rdEnemyShip {
				api_id: ship.api_id,
				name: ship.japanese_name,
				yomi: if ship.reading.is_empty() {
					mst.map(|entry| entry.api_yomi.clone()).unwrap_or_default()
				} else {
					ship.reading
				},
				stype: ship.ship_type,
				ctype: mst.map(|entry| entry.api_ctype).unwrap_or_default(),
				hp: normalize_opt_i64(ship.hp),
				firepower: normalize_opt_i64(ship.firepower),
				torpedo: normalize_opt_i64(ship.torpedo),
				aa: normalize_opt_i64(ship.aa),
				armor: normalize_opt_i64(ship.armor),
				evasion: normalize_opt_i64(ship.evasion),
				asw: normalize_opt_i64(ship.asw),
				los: normalize_opt_i64(ship.los),
				luck: normalize_opt_i64(ship.luck),
				speed: normalize_opt_i64(ship.speed),
				range: normalize_opt_i64(ship.range),
				rarity,
				backs,
				slot_num: ship.equipment.len().min(5) as i64,
				maxeq,
				slots,
			},
		);
	}

	Ok(ships)
}

pub(super) fn parse(
	context: &mut ParseContext,
	manifest: &ApiManifest,
	enemy_path: impl AsRef<Path>,
	enemy_equipment_path: impl AsRef<Path>,
) -> Result<EnemyParsed, ParseError> {
	let enemy_path = enemy_path.as_ref();
	let enemy_equipment_path = enemy_equipment_path.as_ref();
	if !enemy_path.exists() || !enemy_equipment_path.exists() {
		info!(
			"enemy bootstrap sources missing ({} / {}), skipping enemy enrichment",
			enemy_path.display(),
			enemy_equipment_path.display()
		);
		return Ok(EnemyParsed::default());
	}

	let manifest_slotitems = parse_enemy_equipment(context, manifest, enemy_equipment_path)?;
	let ship_map = parse_enemy_ships(context, manifest, enemy_path)?;
	let manifest_ships = ship_map
		.values()
		.map(|ship| build_enemy_manifest_ship(ship, manifest.find_ship(ship.api_id)))
		.collect();

	Ok(EnemyParsed {
		ship_map,
		manifest_ships,
		manifest_slotitems,
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use emukc_model::codex::Codex;

	use std::str::FromStr as _;

	#[test]
	fn parse_enemy_data_enriches_missing_equipment_and_ship_stats() {
		let temp = tempfile::tempdir().unwrap();
		let enemy_path = temp.path().join("enemy.json");
		let enemy_equipment_path = temp.path().join("enemyEquipment.json");
		std::fs::write(
			&enemy_path,
			r#"{
  "Destroyer I-Class": {
    "_api_id": 1501,
    "_japanese_name": "駆逐イ級",
    "_reading": "くちくイきゅう",
    "_type": 2,
    "_hp": 20,
    "_firepower": 5,
    "_torpedo": 15,
    "_aa": 6,
    "_armor": 5,
    "_evasion": 14,
    "_asw": 25,
    "_los": 3,
    "_luck": 1,
    "_speed": 10,
    "_range": 1,
    "_rarity": 1,
    "_back": -1,
    "_equipment": [
      {"equipment": "5inch Single Gun Mount", "size": 0},
      {"equipment": "Abyssal Night Cat Fighter II", "size": 12},
      {"equipment": false, "size": 0}
    ]
  }
}"#,
		)
		.unwrap();
		std::fs::write(
			&enemy_equipment_path,
			r#"{
  "5inch Single Gun Mount": {
    "_id": 1501,
    "_japanese_name": "5inch単装砲",
    "_name": "5inch Single Gun Mount",
    "_type": 1,
    "_firepower": 1,
    "_torpedo": false,
    "_aa": false,
    "_armor": false,
    "_los": false,
    "_asw": false,
    "_range": 1,
    "_evasion": false,
    "_bombing": false,
    "_speed": false
  },
  "Abyssal Night Cat Fighter II": {
    "_id": 601,
    "_japanese_name": "深海夜猫艦戦II",
    "_name": "Abyssal Night Cat Fighter II",
    "_type": 48,
    "_firepower": false,
    "_torpedo": false,
    "_aa": 11,
    "_armor": false,
    "_los": 3,
    "_asw": false,
    "_range": 3,
    "_evasion": 1,
    "_bombing": false,
    "_speed": false
  }
}"#,
		)
		.unwrap();

		let mut context = ParseContext {
			slotitem_name_map: BTreeMap::from([("5inch Single Gun Mount".to_string(), 1501)]),
			useitem_name_map: BTreeMap::new(),
			ship_name_map: BTreeMap::new(),
		};
		let manifest = ApiManifest {
			api_mst_ship: vec![ApiMstShip {
				api_id: 1501,
				api_ctype: 1,
				api_name: "enemy-1501".to_string(),
				api_yomi: "old".to_string(),
				api_stype: 2,
				api_soku: 10,
				api_slot_num: 0,
				api_sort_id: 1501,
				api_backs: Some(2),
				..ApiMstShip::default()
			}],
			api_mst_slotitem: vec![ApiMstSlotitem {
				api_id: 1501,
				api_name: "5inch単装砲".to_string(),
				api_type: [0, 0, 1, 0, 0],
				..ApiMstSlotitem::default()
			}],
			..ApiManifest::default()
		};

		let parsed = parse(&mut context, &manifest, &enemy_path, &enemy_equipment_path).unwrap();
		let ship = parsed.ship_map.get(&1501).unwrap();
		assert_eq!(ship.hp, 20);
		assert_eq!(ship.maxeq, [0, 12, 0, 0, 0]);
		assert_eq!(ship.slots.len(), 2);
		assert_eq!(ship.slots[0].item_id, 1501);
		assert_eq!(ship.slots[1].item_id, 601);

		assert_eq!(parsed.manifest_slotitems.len(), 1);
		assert_eq!(parsed.manifest_slotitems[0].api_id, 601);
		assert_eq!(parsed.manifest_slotitems[0].api_type[2], 48);
		assert_eq!(parsed.manifest_slotitems[0].api_tyku, 11);

		assert_eq!(parsed.manifest_ships.len(), 1);
		assert_eq!(parsed.manifest_ships[0].api_taik, Some([20, 20]));
		assert_eq!(parsed.manifest_ships[0].api_maxeq, Some([0, 12, 0, 0, 0]));
		assert_eq!(parsed.manifest_ships[0].api_slot_num, 3);
		assert_eq!(context.find_slotitem_id("Abyssal Night Cat Fighter II"), Some(601));

		let mut codex = Codex::default();
		codex.manifest = manifest.clone();
		for ship in parsed.manifest_ships.iter().cloned() {
			codex.manifest.api_mst_ship.retain(|existing| existing.api_id != ship.api_id);
			codex.manifest.api_mst_ship.push(ship);
		}
		for slotitem in parsed.manifest_slotitems.iter().cloned() {
			codex.manifest.api_mst_slotitem.retain(|existing| existing.api_id != slotitem.api_id);
			codex.manifest.api_mst_slotitem.push(slotitem);
		}
		codex.enemy_ship_extra = parsed.ship_map.clone();

		let save_root = tempfile::tempdir().unwrap();
		codex.save(save_root.path(), true).unwrap();
		assert!(save_root.path().join("enemy_ship_extra.json").exists());

		let loaded = Codex::load_without_cache_source(save_root.path()).unwrap();
		let loaded_enemy = loaded.enemy_ship_extra.get(&1501).unwrap();
		assert_eq!(loaded_enemy.hp, 20);
		assert_eq!(loaded_enemy.maxeq, [0, 12, 0, 0, 0]);
		assert_eq!(loaded.manifest.find_ship(1501).unwrap().api_taik, Some([20, 20]));
	}

	/// Verify that real kcwiki data produces valid enemy entries for all map-referenced enemies.
	/// Skipped if bootstrap data files are not present.
	#[test]
	fn real_kcwiki_enemy_coverage_for_map_compositions() {
		let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.parent()
			.unwrap()
			.parent()
			.unwrap()
			.join(".data/temp");
		let enemy_path = data_dir.join("kcwiki_enemy.json");
		let enemy_equipment_path = data_dir.join("kcwiki_enemy_equipment.json");
		let manifest_path = data_dir.parent().unwrap().join("codex").join("start2.json");

		if !enemy_path.exists() || !enemy_equipment_path.exists() || !manifest_path.exists() {
			eprintln!("Skipping real kcwiki enemy coverage test (data files not present)");
			return;
		}

		let raw = std::fs::read_to_string(&manifest_path).unwrap();
		let manifest = ApiManifest::from_str(&raw).unwrap();
		let mut context = super::super::prepare_context(&data_dir).unwrap();

		let parsed = parse(&mut context, &manifest, &enemy_path, &enemy_equipment_path).unwrap();
		assert!(
			parsed.ship_map.len() > 800,
			"expected 800+ enemies, got {}",
			parsed.ship_map.len()
		);

		// Build codex with enemy data
		let mut codex = Codex::default();
		codex.manifest = manifest;
		for ship in parsed.manifest_ships.iter().cloned() {
			codex.manifest.api_mst_ship.retain(|e| e.api_id != ship.api_id);
			codex.manifest.api_mst_ship.push(ship);
		}
		for slotitem in parsed.manifest_slotitems.iter().cloned() {
			codex.manifest.api_mst_slotitem.retain(|e| e.api_id != slotitem.api_id);
			codex.manifest.api_mst_slotitem.push(slotitem);
		}
		codex.enemy_ship_extra = parsed.ship_map.clone();

		// Verify new_enemy_ship returns Tier 1 for every parsed enemy
		let mut tier1_count = 0;
		for (&ship_id, enemy_data) in &parsed.ship_map {
			let result = codex.new_enemy_ship(ship_id);
			assert!(
				result.is_some(),
				"new_enemy_ship({ship_id}) returned None for {}",
				enemy_data.name
			);
			let (ship, slot_items) = result.unwrap();
			assert_eq!(ship.api_nowhp, enemy_data.hp, "HP mismatch for {ship_id}");
			assert_eq!(ship.api_karyoku[0], enemy_data.firepower, "FP mismatch for {ship_id}");
			assert_eq!(ship.api_soukou[0], enemy_data.armor, "Armor mismatch for {ship_id}");
			assert_eq!(
				slot_items.len(),
				enemy_data.slots.len(),
				"Slot count mismatch for {ship_id}"
			);
			tier1_count += 1;
		}

		eprintln!(
			"Tier 1 enemy coverage: {tier1_count}/{} parsed enemies verified",
			parsed.ship_map.len()
		);
	}
}
