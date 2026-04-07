use std::{
	collections::{BTreeMap, BTreeSet},
	fs,
	path::Path,
};

use emukc_model::{
	codex::map::{
		EnemyComposition, EnemyFleetDefinition, MapCatalog, MapCellDefinition, MapDefinition,
		MapResetPolicy, MapVariantDefinition,
	},
	kc2::start2::{ApiManifest, ApiMstMapinfo},
};
use serde::Deserialize;
use serde_yaml::Deserializer;

use crate::parser::error::ParseError;

#[derive(Debug, Clone, Default, Deserialize)]
struct RawMapInfoJson {
	#[serde(default)]
	spots: Vec<RawSpot>,
	#[serde(default)]
	enemies: Vec<RawEnemyMarker>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawSpot {
	no: i64,
	#[serde(default)]
	color: Option<i64>,
	#[serde(default)]
	replenish: Option<serde_json::Value>,
	#[serde(default)]
	ration: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct RawEnemyMarker {
	no: i64,
	img: String,
}

#[derive(Debug, Clone, Deserialize)]
struct KcDataMapYaml {
	data: KcDataMapData,
}

#[derive(Debug, Clone, Deserialize)]
struct KcDataMapData {
	id: i64,
	name: String,
	#[serde(default)]
	routes: BTreeMap<i64, KcDataRoute>,
	#[serde(default)]
	cells: BTreeMap<String, KcDataCell>,
}

#[derive(Debug, Clone, Deserialize)]
struct KcDataRoute {
	from: Option<KcDataNode>,
	to: KcDataNode,
}

#[derive(Debug, Clone, Deserialize)]
struct KcDataCell {
	#[serde(default)]
	name: String,
	#[serde(default)]
	boss: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum KcDataNode {
	Int(i64),
	String(String),
}

#[allow(dead_code)]
pub(super) fn load_map_catalog_from_cache_root(
	cache_root: impl AsRef<Path>,
	manifest: &ApiManifest,
) -> Result<MapCatalog, ParseError> {
	let mut catalog = MapCatalog::from_manifest(manifest);
	let map_root = cache_root.as_ref().join("kcs2/resources/map");
	let Ok(area_entries) = fs::read_dir(&map_root) else {
		return Ok(catalog);
	};

	let mut raw_variants: BTreeMap<(i64, String), RawMapInfoJson> = BTreeMap::new();

	for area_entry in area_entries {
		let area_entry = area_entry.map_err(|source| ParseError::io_at(&map_root, source))?;
		let path = area_entry.path();
		if !path.is_dir() {
			continue;
		}
		let Some(area_name) = path.file_name().and_then(|name| name.to_str()) else {
			continue;
		};
		let Ok(maparea_id) = area_name.parse::<i64>() else {
			continue;
		};
		let files = fs::read_dir(&path).map_err(|source| ParseError::io_at(&path, source))?;

		for file in files {
			let file = file.map_err(|source| ParseError::io_at(&path, source))?;
			let file_path = file.path();
			if file_path.extension().and_then(|ext| ext.to_str()) != Some("json") {
				continue;
			}
			let Some(stem) = file_path.file_stem().and_then(|name| name.to_str()) else {
				continue;
			};
			let Some((mapinfo_no, variant_key)) = parse_map_info_stem(stem) else {
				continue;
			};
			let raw = fs::read_to_string(&file_path)
				.map_err(|source| ParseError::io_at(&file_path, source))?;
			let parsed = serde_json::from_str::<RawMapInfoJson>(&raw)
				.map_err(|source| ParseError::json_at(&file_path, source))?;
			raw_variants.insert((compose_map_id(maparea_id, mapinfo_no), variant_key), parsed);
		}
	}

	let mut per_map: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
	for (map_id, variant_key) in raw_variants.keys() {
		per_map.entry(*map_id).or_default().insert(variant_key.clone());
	}

	for (map_id, variant_keys) in per_map {
		let (maparea_id, mapinfo_no) = split_map_id(map_id);
		let manifest_map =
			manifest.api_mst_mapinfo.iter().find(|map| map.api_id == map_id).cloned();
		let area_type = manifest
			.api_mst_maparea
			.iter()
			.find(|area| area.api_id == maparea_id)
			.map(|area| area.api_type)
			.unwrap_or(if maparea_id > 7 {
				1
			} else {
				0
			});
		let entry = catalog.maps.entry(map_id).or_insert_with(|| MapDefinition {
			map_id,
			maparea_id,
			mapinfo_no,
			name: manifest_map
				.as_ref()
				.map(|map| map.api_name.clone())
				.unwrap_or_else(|| format!("Event {maparea_id}-{mapinfo_no}")),
			level: manifest_map.as_ref().map(|map| map.api_level).unwrap_or(1),
			sally_flag: manifest_map
				.as_ref()
				.map(|map| map.api_sally_flag.clone())
				.unwrap_or_default(),
			is_event: area_type == 1,
			reset_policy: MapResetPolicy::Never,
			airbase_count: None,
			gauge_type: None,
			gauge_count: None,
			required_defeat_count: manifest_map
				.as_ref()
				.and_then(|map| map.api_required_defeat_count),
			max_hp: manifest_map.as_ref().and_then(extract_max_hp),
			default_variant: String::new(),
			rank_stage_ids: BTreeMap::new(),
			variants: BTreeMap::new(),
		});

		for variant_key in variant_keys {
			let merged = merge_raw_variant(
				raw_variants.get(&(map_id, String::new())),
				raw_variants.get(&(map_id, variant_key.clone())),
			);
			let variant = build_variant_definition(map_id, &variant_key, merged, manifest);
			entry.variants.insert(variant_key.clone(), variant);
		}
	}

	Ok(catalog)
}

pub(super) fn load_map_catalog_from_kcdata_root(
	kcdata_root: impl AsRef<Path>,
	manifest: &ApiManifest,
) -> Result<MapCatalog, ParseError> {
	let mut catalog = MapCatalog::from_manifest(manifest);
	let map_root = kcdata_root.as_ref().join("_map");
	let Ok(entries) = fs::read_dir(&map_root) else {
		return Ok(catalog);
	};

	for entry in entries {
		let entry = entry.map_err(|source| ParseError::io_at(&map_root, source))?;
		let path = entry.path();
		if !path.is_file() {
			continue;
		}
		let raw = fs::read_to_string(&path).map_err(|source| ParseError::io_at(&path, source))?;

		let mut parsed = None;
		for doc in Deserializer::from_str(&raw) {
			match KcDataMapYaml::deserialize(doc) {
				Ok(map) => {
					parsed = Some(map.data);
					break;
				}
				Err(_) => continue,
			}
		}
		let Some(parsed) = parsed else {
			continue;
		};

		let (maparea_id, mapinfo_no) = split_map_id(parsed.id);
		let manifest_map =
			manifest.api_mst_mapinfo.iter().find(|map| map.api_id == parsed.id).cloned();
		let entry = catalog.maps.entry(parsed.id).or_insert_with(|| MapDefinition {
			map_id: parsed.id,
			maparea_id,
			mapinfo_no,
			name: parsed.name.clone(),
			level: manifest_map.as_ref().map(|map| map.api_level).unwrap_or(1),
			sally_flag: manifest_map
				.as_ref()
				.map(|map| map.api_sally_flag.clone())
				.unwrap_or_default(),
			is_event: maparea_id > 7,
			reset_policy: MapResetPolicy::Never,
			airbase_count: None,
			gauge_type: None,
			gauge_count: None,
			required_defeat_count: manifest_map
				.as_ref()
				.and_then(|map| map.api_required_defeat_count),
			max_hp: manifest_map.as_ref().and_then(extract_max_hp),
			default_variant: String::new(),
			rank_stage_ids: BTreeMap::new(),
			variants: BTreeMap::new(),
		});

		entry.name = parsed.name.clone();
		entry.variants.insert(String::new(), build_variant_from_kcdata(&parsed));
	}

	Ok(catalog)
}

fn compose_map_id(maparea_id: i64, mapinfo_no: i64) -> i64 {
	maparea_id * 10 + mapinfo_no
}

fn split_map_id(map_id: i64) -> (i64, i64) {
	(map_id / 10, map_id % 10)
}

fn parse_map_info_stem(stem: &str) -> Option<(i64, String)> {
	let (map_part, variant_part) = stem.split_once("_info")?;
	let mapinfo_no = map_part.parse::<i64>().ok()?;
	Some((mapinfo_no, variant_part.to_string()))
}

fn extract_max_hp(map: &ApiMstMapinfo) -> Option<i64> {
	match map.api_max_maphp.as_ref()? {
		serde_json::Value::Number(value) => value.as_i64(),
		serde_json::Value::String(value) => value.parse::<i64>().ok(),
		serde_json::Value::Array(values) => values.first().and_then(|value| match value {
			serde_json::Value::Number(number) => number.as_i64(),
			serde_json::Value::String(string) => string.parse::<i64>().ok(),
			_ => None,
		}),
		_ => None,
	}
}

fn merge_raw_variant(
	base: Option<&RawMapInfoJson>,
	overlay: Option<&RawMapInfoJson>,
) -> RawMapInfoJson {
	let mut spots = BTreeMap::new();
	let mut enemies = BTreeMap::new();

	if let Some(base) = base {
		for spot in &base.spots {
			spots.insert(spot.no, spot.clone());
		}
		for enemy in &base.enemies {
			enemies.insert(enemy.no, enemy.clone());
		}
	}

	if let Some(overlay) = overlay {
		for spot in &overlay.spots {
			spots.insert(spot.no, spot.clone());
		}
		for enemy in &overlay.enemies {
			enemies.insert(enemy.no, enemy.clone());
		}
	}

	RawMapInfoJson {
		spots: spots.into_values().collect(),
		enemies: enemies.into_values().collect(),
	}
}

fn build_variant_definition(
	map_id: i64,
	variant_key: &str,
	raw: RawMapInfoJson,
	manifest: &ApiManifest,
) -> MapVariantDefinition {
	let mut spots = raw.spots;
	spots.sort_by_key(|spot| spot.no);
	let battle_cells = raw
		.enemies
		.iter()
		.filter_map(|enemy| {
			parse_icon_ship_id(&enemy.img).map(|ship_id| {
				(
					enemy.no,
					EnemyFleetDefinition {
						cell_no: enemy.no,
						battle_kind: 1,
						formations: vec![1],
						compositions: vec![EnemyComposition {
							comp_id: format!("{map_id}:{variant_key}:{}", enemy.no),
							weight: 1,
							ship_ids: vec![if manifest
								.api_mst_ship
								.iter()
								.any(|ship| ship.api_id == ship_id)
							{
								ship_id
							} else {
								412
							}],
							formation: Some(1),
							raw_ship_names: Vec::new(),
						}],
					},
				)
			})
		})
		.collect::<BTreeMap<_, _>>();
	let boss_cell_no = battle_cells
		.keys()
		.max()
		.copied()
		.or_else(|| spots.iter().map(|spot| spot.no).filter(|no| *no > 0).max())
		.unwrap_or(1);
	let nonzero_cells = spots.iter().map(|spot| spot.no).filter(|no| *no > 0).collect::<Vec<_>>();
	let mut cells = Vec::with_capacity(spots.len());

	for (idx, spot) in spots.iter().enumerate() {
		let next_cells = if spot.no == 0 {
			nonzero_cells.first().copied().into_iter().collect()
		} else {
			spots
				.iter()
				.skip(idx + 1)
				.find(|candidate| candidate.no > spot.no)
				.map(|candidate| vec![candidate.no])
				.unwrap_or_default()
		};
		let has_enemy = battle_cells.contains_key(&spot.no);
		let is_boss = has_enemy && spot.no == boss_cell_no;
		let is_anchorage = spot.replenish.is_some() || spot.ration.is_some();
		let (color_no, event_id, event_kind) = if spot.no == 0 {
			(0, 0, 0)
		} else if is_boss {
			(5, 5, 1)
		} else if has_enemy {
			(4, 4, 1)
		} else if is_anchorage {
			(14, 10, 0)
		} else {
			(spot.color.filter(|color| *color > 0).unwrap_or(6), 1, 0)
		};

		cells.push(MapCellDefinition {
			cell_no: spot.no,
			color_no,
			event_id,
			event_kind,
			next_cells,
			node_label: if spot.no == 0 {
				Some("Start".to_string())
			} else {
				Some(spot.no.to_string())
			},
			master_cell_id: None,
			distance: None,
		});
	}

	MapVariantDefinition {
		variant_key: variant_key.to_string(),
		boss_cell_no,
		cells,
		routing_rules: BTreeMap::new(),
		enemy_fleets: battle_cells,
		ship_drops: BTreeMap::new(),
		required_defeat_count: None,
		clear_to_variant_key: None,
		parse_warnings: Vec::new(),
	}
}

fn parse_icon_ship_id(img: &str) -> Option<i64> {
	img.strip_prefix("icon_")?.parse::<i64>().ok()
}

fn build_variant_from_kcdata(data: &KcDataMapData) -> MapVariantDefinition {
	let actual_node_keys = data.cells.keys().cloned().collect::<BTreeSet<_>>();
	let route_graph = data
		.routes
		.values()
		.filter_map(|route| {
			route.from.as_ref().and_then(route_node_key).zip(route_node_key(&route.to))
		})
		.collect::<Vec<_>>();
	let route_targets =
		route_graph.iter().fold(BTreeMap::<String, Vec<String>>::new(), |mut acc, (from, to)| {
			acc.entry(from.clone()).or_default().push(to.clone());
			acc
		});
	let mut assigned_numbers = BTreeMap::new();
	let mut used_numbers = BTreeSet::new();
	for key in &actual_node_keys {
		if let Ok(value) = key.parse::<i64>()
			&& value >= 0
		{
			assigned_numbers.insert(key.to_string(), value);
			used_numbers.insert(value);
		}
	}

	let mut next_no = 1_i64;
	for key in ordered_kcdata_nodes(data) {
		if !actual_node_keys.contains(&key) {
			continue;
		}
		if assigned_numbers.contains_key(&key) {
			continue;
		}
		while used_numbers.contains(&next_no) {
			next_no += 1;
		}
		assigned_numbers.insert(key, next_no);
		used_numbers.insert(next_no);
		next_no += 1;
	}

	let start_targets = resolve_kcdata_targets(
		data.routes
			.values()
			.filter(|route| route.from.is_none())
			.filter_map(|route| route_node_key(&route.to))
			.collect::<Vec<_>>(),
		&actual_node_keys,
		&route_targets,
		&assigned_numbers,
	);

	let mut cells = Vec::with_capacity(data.cells.len() + 1);
	cells.push(MapCellDefinition {
		cell_no: 0,
		color_no: 0,
		event_id: 0,
		event_kind: 0,
		next_cells: start_targets,
		node_label: Some("Start".to_string()),
		master_cell_id: None,
		distance: None,
	});

	let mut boss_cell_no = 1;
	for key in ordered_kcdata_nodes(data) {
		if !actual_node_keys.contains(&key) {
			continue;
		}
		let cell = data.cells.get(&key);
		let cell_no = assigned_numbers[&key];
		let next_cells = resolve_kcdata_targets(
			route_targets.get(&key).cloned().unwrap_or_default(),
			&actual_node_keys,
			&route_targets,
			&assigned_numbers,
		);
		let has_battle = cell.is_some_and(|cell| cell.boss || !cell.name.trim().is_empty());
		let (color_no, event_id, event_kind) = if cell.is_some_and(|cell| cell.boss) {
			boss_cell_no = cell_no;
			(5, 5, 1)
		} else if has_battle {
			(4, 4, 1)
		} else {
			(6, 1, 0)
		};

		cells.push(MapCellDefinition {
			cell_no,
			color_no,
			event_id,
			event_kind,
			next_cells,
			node_label: Some(key.clone()),
			master_cell_id: None,
			distance: None,
		});
	}

	MapVariantDefinition {
		variant_key: String::new(),
		boss_cell_no,
		cells,
		routing_rules: BTreeMap::new(),
		enemy_fleets: BTreeMap::new(),
		ship_drops: BTreeMap::new(),
		required_defeat_count: None,
		clear_to_variant_key: None,
		parse_warnings: Vec::new(),
	}
}

fn collect_kcdata_nodes(data: &KcDataMapData) -> BTreeSet<String> {
	let mut nodes = data.cells.keys().cloned().collect::<BTreeSet<_>>();
	for route in data.routes.values() {
		if let Some(from) = route.from.as_ref().and_then(route_node_key) {
			nodes.insert(from);
		}
		if let Some(to) = route_node_key(&route.to) {
			nodes.insert(to);
		}
	}
	nodes
}

fn ordered_kcdata_nodes(data: &KcDataMapData) -> Vec<String> {
	let mut ordered = Vec::new();
	let mut seen = BTreeSet::new();
	let mut queue = data
		.routes
		.values()
		.filter(|route| route.from.is_none())
		.filter_map(|route| route_node_key(&route.to))
		.collect::<Vec<_>>();

	while let Some(key) = queue.pop() {
		if !seen.insert(key.clone()) {
			continue;
		}
		ordered.push(key.clone());
		for route in data.routes.values() {
			if route.from.as_ref().and_then(route_node_key).as_ref() == Some(&key)
				&& let Some(next_key) = route_node_key(&route.to)
			{
				queue.insert(0, next_key);
			}
		}
	}

	for key in collect_kcdata_nodes(data) {
		if seen.insert(key.clone()) {
			ordered.push(key);
		}
	}

	ordered
}

fn resolve_kcdata_targets(
	start_keys: Vec<String>,
	actual_node_keys: &BTreeSet<String>,
	route_targets: &BTreeMap<String, Vec<String>>,
	assigned_numbers: &BTreeMap<String, i64>,
) -> Vec<i64> {
	let mut resolved = Vec::new();
	let mut visited = BTreeSet::new();
	let mut stack = start_keys.into_iter().rev().collect::<Vec<_>>();

	while let Some(key) = stack.pop() {
		if !visited.insert(key.clone()) {
			continue;
		}
		if actual_node_keys.contains(&key) {
			if let Some(cell_no) = assigned_numbers.get(&key).copied()
				&& !resolved.contains(&cell_no)
			{
				resolved.push(cell_no);
			}
			continue;
		}

		if let Some(next_keys) = route_targets.get(&key) {
			for next_key in next_keys.iter().rev() {
				stack.push(next_key.clone());
			}
		}
	}

	resolved
}

fn route_node_key(node: &KcDataNode) -> Option<String> {
	match node {
		KcDataNode::Int(value) => Some(value.to_string()),
		KcDataNode::String(value) => Some(value.clone()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parse_kcdata_map(raw: &str) -> KcDataMapData {
		for doc in Deserializer::from_str(raw) {
			if let Ok(map) = KcDataMapYaml::deserialize(doc) {
				return map.data;
			}
		}
		panic!("kcdata yaml should parse");
	}

	#[test]
	fn build_variant_from_kcdata_skips_route_only_numeric_placeholders() {
		let raw = r#"---
layout: json
order: 11
data:
  id: 11
  name: 鎮守府正面海域
  routes:
    0:
      from: null
      to: 1
    1:
      from: 1
      to: A
    2:
      from: A
      to: B
    3:
      from: A
      to: C
  cells:
    A:
      name: 敵偵察艦
      type:
      boss: false
      routes: [1]
    B:
      name: 敵はぐれ艦隊
      type:
      boss: false
      routes: [2]
    C:
      name: 敵主力艦隊
      type:
      boss: true
      routes: [3]
---"#;
		let data = parse_kcdata_map(raw);

		let variant = build_variant_from_kcdata(&data);
		let cell_nos = variant.cells.iter().map(|cell| cell.cell_no).collect::<Vec<_>>();

		assert_eq!(cell_nos, vec![0, 1, 2, 3]);
		assert_eq!(variant.cells[0].next_cells, vec![1]);
		assert_eq!(variant.cells[1].next_cells, vec![2, 3]);
		assert_eq!(variant.boss_cell_no, 3);
	}

	#[test]
	fn build_variant_from_all_repo_kcdata_maps_keeps_real_cell_count_and_valid_edges() {
		let map_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../.data/temp/kc_data/_map");
		let mut entries = std::fs::read_dir(&map_root)
			.unwrap()
			.flatten()
			.map(|entry| entry.path())
			.collect::<Vec<_>>();
		entries.sort();

		for path in entries {
			if !path.is_file() {
				continue;
			}
			let raw = std::fs::read_to_string(&path).unwrap();
			let data = parse_kcdata_map(&raw);
			let variant = build_variant_from_kcdata(&data);
			let cell_nos = variant.cells.iter().map(|cell| cell.cell_no).collect::<BTreeSet<_>>();

			assert_eq!(
				variant.cells.len(),
				data.cells.len() + 1,
				"{} produced unexpected cell count",
				path.display()
			);
			assert!(
				cell_nos.contains(&0),
				"{} is missing the synthetic start cell",
				path.display()
			);

			for cell in &variant.cells {
				for next in &cell.next_cells {
					assert!(
						cell_nos.contains(next),
						"{} has dangling edge {} -> {}",
						path.display(),
						cell.cell_no,
						next
					);
				}
			}
		}
	}

	#[test]
	fn build_variant_from_all_repo_kcdata_maps_preserves_real_numeric_cell_keys() {
		let map_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../.data/temp/kc_data/_map");
		let mut entries = std::fs::read_dir(&map_root)
			.unwrap()
			.flatten()
			.map(|entry| entry.path())
			.collect::<Vec<_>>();
		entries.sort();

		for path in entries {
			if !path.is_file() {
				continue;
			}
			let raw = std::fs::read_to_string(&path).unwrap();
			let data = parse_kcdata_map(&raw);
			let variant = build_variant_from_kcdata(&data);
			let cell_nos = variant.cells.iter().map(|cell| cell.cell_no).collect::<BTreeSet<_>>();

			for numeric_key in data.cells.keys().filter_map(|key| key.parse::<i64>().ok()) {
				assert!(
					cell_nos.contains(&numeric_key),
					"{} lost real numeric cell {}",
					path.display(),
					numeric_key
				);
			}
		}
	}
}
