#![allow(missing_docs)]

use std::{
	collections::{BTreeMap, BTreeSet},
	fs,
	path::Path,
};

use serde::{Deserialize, Serialize};
use serde_yaml::Deserializer;

use crate::{
	kc2::start2::{ApiManifest, ApiMstMapinfo},
	profile::map_record::{DEFAULT_MAP_RECORDS, MapRefreshType},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, enumn::N)]
pub enum MapResetPolicy {
	#[default]
	Never = 0,
	Monthly = 1,
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
	pub variants: BTreeMap<String, MapVariantDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapVariantDefinition {
	pub variant_key: String,
	pub boss_cell_no: i64,
	pub cells: Vec<MapCellDefinition>,
	pub enemy_fleets: BTreeMap<i64, EnemyFleetDefinition>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MapCellDefinition {
	pub cell_no: i64,
	pub color_no: i64,
	pub event_id: i64,
	pub event_kind: i64,
	pub next_cells: Vec<i64>,
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
}

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

impl MapCatalog {
	pub fn from_manifest(manifest: &ApiManifest) -> Self {
		let defaults = DEFAULT_MAP_RECORDS
			.iter()
			.map(|record| (record.id, record))
			.collect::<BTreeMap<_, _>>();
		let mut maps = BTreeMap::new();
		let area_types = manifest
			.api_mst_maparea
			.iter()
			.map(|area| (area.api_id, area.api_type))
			.collect::<BTreeMap<_, _>>();

		for map in &manifest.api_mst_mapinfo {
			let overlay = defaults.get(&map.api_id);
			let (reset_policy, airbase_count, gauge_type, gauge_count, required_defeat_count) =
				if let Some(record) = overlay {
					let defeat_ctx = record.defeat_ctx.as_ref();
					(
						defeat_ctx
							.map(|ctx| match ctx.refresh_type {
								MapRefreshType::Monthly => MapResetPolicy::Monthly,
								MapRefreshType::Never => MapResetPolicy::Never,
							})
							.unwrap_or(MapResetPolicy::Never),
						record.airbase_count,
						defeat_ctx.map(|ctx| ctx.gauge_type as i64),
						defeat_ctx.map(|ctx| ctx.gauge_num),
						defeat_ctx.map(|ctx| ctx.defeat_required),
					)
				} else {
					(MapResetPolicy::Never, None, None, None, map.api_required_defeat_count)
				};
			let is_event = area_types.get(&map.api_maparea_id).copied().unwrap_or_default() == 1;
			maps.insert(
				map.api_id,
				MapDefinition {
					map_id: map.api_id,
					maparea_id: map.api_maparea_id,
					mapinfo_no: map.api_no,
					name: map.api_name.clone(),
					level: map.api_level,
					sally_flag: map.api_sally_flag.clone(),
					is_event,
					reset_policy,
					airbase_count,
					gauge_type,
					gauge_count,
					required_defeat_count,
					max_hp: extract_max_hp(map),
					default_variant: String::new(),
					variants: BTreeMap::new(),
				},
			);
		}

		let mut catalog = Self {
			maps,
		};
		catalog.ensure_synthetic_variants();
		catalog
	}

	pub fn load_from_cache_root(cache_root: impl AsRef<Path>, manifest: &ApiManifest) -> Self {
		let mut catalog = Self::from_manifest(manifest);
		let map_root = cache_root.as_ref().join("kcs2/resources/map");
		let Ok(area_entries) = fs::read_dir(&map_root) else {
			return catalog;
		};

		let mut raw_variants: BTreeMap<(i64, String), RawMapInfoJson> = BTreeMap::new();

		for area_entry in area_entries.flatten() {
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
			let Ok(files) = fs::read_dir(&path) else {
				continue;
			};

			for file in files.flatten() {
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
				let Ok(raw) = fs::read_to_string(&file_path) else {
					continue;
				};
				let Ok(parsed) = serde_json::from_str::<RawMapInfoJson>(&raw) else {
					continue;
				};
				raw_variants.insert((compose_map_id(maparea_id, mapinfo_no), variant_key), parsed);
			}
		}

		let mut per_map: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
		for ((map_id, variant_key), _) in &raw_variants {
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

		catalog.ensure_synthetic_variants();
		catalog
	}

	pub fn load_from_kcdata_root(kcdata_root: impl AsRef<Path>, manifest: &ApiManifest) -> Self {
		let mut catalog = Self::from_manifest(manifest);
		let map_root = kcdata_root.as_ref().join("_map");
		let Ok(entries) = fs::read_dir(&map_root) else {
			return catalog;
		};

		for entry in entries.flatten() {
			let path = entry.path();
			if !path.is_file() {
				continue;
			}
			let Ok(raw) = fs::read_to_string(&path) else {
				continue;
			};

			let mut parsed = None;
			for doc in Deserializer::from_str(&raw) {
				if let Ok(map) = KcDataMapYaml::deserialize(doc) {
					parsed = Some(map.data);
					break;
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
				variants: BTreeMap::new(),
			});

			entry.name = parsed.name.clone();
			entry.variants.insert(String::new(), build_variant_from_kcdata(&parsed));
		}

		catalog.ensure_synthetic_variants();
		catalog
	}

	pub fn map_definition(&self, map_id: i64) -> Option<&MapDefinition> {
		self.maps.get(&map_id)
	}

	pub fn map_definition_by_area_no(
		&self,
		maparea_id: i64,
		mapinfo_no: i64,
	) -> Option<&MapDefinition> {
		self.maps.get(&compose_map_id(maparea_id, mapinfo_no))
	}

	pub fn known_maps(&self) -> Vec<&MapDefinition> {
		self.maps.values().collect()
	}

	fn ensure_synthetic_variants(&mut self) {
		for definition in self.maps.values_mut() {
			if definition.variants.is_empty() {
				definition.variants.insert(
					String::new(),
					MapVariantDefinition {
						variant_key: String::new(),
						boss_cell_no: 1,
						cells: vec![
							MapCellDefinition {
								cell_no: 0,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: vec![1],
							},
							MapCellDefinition {
								cell_no: 1,
								color_no: 5,
								event_id: 5,
								event_kind: 1,
								next_cells: vec![],
							},
						],
						enemy_fleets: BTreeMap::new(),
					},
				);
			}
		}
	}
}

impl MapDefinition {
	pub fn variant(&self, variant_key: &str) -> Option<&MapVariantDefinition> {
		self.variants
			.get(variant_key)
			.or_else(|| self.variants.get(&self.default_variant))
			.or_else(|| self.variants.get(""))
	}
}

impl MapVariantDefinition {
	pub fn cell(&self, cell_no: i64) -> Option<&MapCellDefinition> {
		self.cells.iter().find(|cell| cell.cell_no == cell_no)
	}

	pub fn first_progress_cell_no(&self) -> Option<i64> {
		self.cell(0)
			.and_then(|start| start.next_cells.first().copied())
			.or_else(|| self.cells.iter().find(|cell| cell.cell_no > 0).map(|cell| cell.cell_no))
	}
}

fn compose_map_id(maparea_id: i64, mapinfo_no: i64) -> i64 {
	maparea_id * 10 + mapinfo_no
}

pub fn split_map_id(map_id: i64) -> (i64, i64) {
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
		});
	}

	MapVariantDefinition {
		variant_key: variant_key.to_string(),
		boss_cell_no,
		cells,
		enemy_fleets: battle_cells,
	}
}

fn parse_icon_ship_id(img: &str) -> Option<i64> {
	img.strip_prefix("icon_")?.parse::<i64>().ok()
}

fn build_variant_from_kcdata(data: &KcDataMapData) -> MapVariantDefinition {
	let route_graph = data
		.routes
		.values()
		.filter_map(|route| {
			route.from.as_ref().and_then(route_node_key).zip(route_node_key(&route.to))
		})
		.collect::<Vec<_>>();
	let all_node_keys = collect_kcdata_nodes(data);
	let mut assigned_numbers = BTreeMap::new();
	let mut used_numbers = BTreeSet::new();
	for key in &all_node_keys {
		if let Ok(value) = key.parse::<i64>() {
			if value >= 0 {
				assigned_numbers.insert(key.to_string(), value);
				used_numbers.insert(value);
			}
		}
	}

	let mut next_no = 1_i64;
	for key in ordered_kcdata_nodes(data) {
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

	let start_targets = data
		.routes
		.values()
		.filter(|route| route.from.is_none())
		.filter_map(|route| route_node_key(&route.to))
		.filter_map(|key| assigned_numbers.get(&key).copied())
		.collect::<Vec<_>>();

	let mut cells = Vec::with_capacity(data.cells.len() + 1);
	cells.push(MapCellDefinition {
		cell_no: 0,
		color_no: 0,
		event_id: 0,
		event_kind: 0,
		next_cells: start_targets,
	});

	let mut boss_cell_no = 1;
	for key in ordered_kcdata_nodes(data) {
		let cell = data.cells.get(&key);
		let cell_no = assigned_numbers[&key];
		let next_cells = route_graph
			.iter()
			.filter(|(from, _)| *from == key)
			.filter_map(|(_, to_key)| assigned_numbers.get(to_key).copied())
			.collect::<Vec<_>>();
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
		});
	}

	MapVariantDefinition {
		variant_key: String::new(),
		boss_cell_no,
		cells,
		enemy_fleets: BTreeMap::new(),
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

fn route_node_key(node: &KcDataNode) -> Option<String> {
	match node {
		KcDataNode::Int(value) => Some(value.to_string()),
		KcDataNode::String(value) => Some(value.clone()),
	}
}
