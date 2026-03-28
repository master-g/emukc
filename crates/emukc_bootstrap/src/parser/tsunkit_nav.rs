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
	kc2::start2::ApiManifest,
};
use serde::Deserialize;

use super::error::ParseError;

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitMapResponse {
	result: TsunkitMapResult,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitMapResult {
	#[serde(default)]
	route: BTreeMap<String, (Option<String>, String, i64, i64)>,
	#[serde(default)]
	spots: BTreeMap<String, (i64, i64, Option<String>)>,
	#[serde(default, rename = "mapSet")]
	map_set: TsunkitMapSet,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitMapSet {
	#[serde(default)]
	map: TsunkitMapLayer,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitMapLayer {
	#[serde(default)]
	spots: Vec<TsunkitRenderedSpot>,
	#[serde(default)]
	enemies: Vec<TsunkitRenderedEnemyMarker>,
	#[serde(default)]
	airraids: Vec<TsunkitAirRaidMarker>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitRenderedSpot {
	x: i64,
	y: i64,
	#[serde(default)]
	no: Option<i64>,
	#[serde(default)]
	color: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitRenderedEnemyMarker {
	no: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitAirRaidMarker {
	#[serde(default)]
	no: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitEnemyCompsResponse {
	result: TsunkitEnemyCompsResult,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitEnemyCompsResult {
	#[serde(default)]
	entries: Vec<TsunkitEnemyCompEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitEnemyCompEntry {
	#[serde(default)]
	node: String,
	#[serde(default, rename = "mainFleet")]
	main_fleet: Vec<TsunkitEnemyShip>,
	#[serde(default, rename = "escortFleet")]
	escort_fleet: Vec<TsunkitEnemyShip>,
	#[serde(default)]
	formation: i64,
	#[serde(default)]
	count: i64,
	#[serde(default, rename = "masterId")]
	master_id: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitEnemyShip {
	id: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitNodeSummaryResponse {
	#[serde(default)]
	result: BTreeMap<String, TsunkitNodeSummaryEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TsunkitNodeSummaryEntry {
	#[serde(default)]
	battles: i64,
}

/// Parse a cached `tsunkit_nav` directory into a normalized [`MapCatalog`].
pub fn parse(root: impl AsRef<Path>, manifest: &ApiManifest) -> Result<MapCatalog, ParseError> {
	let root = root.as_ref();
	let map_root = root.join("maps");
	let nodesummary_root = root.join("nodesummary");
	let enemy_root = root.join("enemycomps");
	let entries = fs::read_dir(&map_root).map_err(|source| ParseError::io_at(&map_root, source))?;

	let mut catalog = MapCatalog::default();

	for entry in entries.flatten() {
		let path = entry.path();
		if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("json") {
			continue;
		}
		let Some(map_name) = path.file_stem().and_then(|name| name.to_str()) else {
			continue;
		};
		let Some((maparea_id, mapinfo_no)) = parse_map_name(map_name) else {
			continue;
		};
		let parsed = load_json::<TsunkitMapResponse>(&path)?;
		let map_id = maparea_id * 10 + mapinfo_no;
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

		let definition = catalog.maps.entry(map_id).or_insert_with(|| MapDefinition {
			map_id,
			maparea_id,
			mapinfo_no,
			name: manifest_map
				.as_ref()
				.map(|map| map.api_name.clone())
				.unwrap_or_else(|| map_name.to_string()),
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
			max_hp: None,
			default_variant: String::new(),
			variants: BTreeMap::new(),
		});

		let variant = build_variant(
			map_name,
			&parsed.result,
			load_optional_json::<TsunkitNodeSummaryResponse>(
				&nodesummary_root.join(format!("{map_name}.json")),
			)?,
			&enemy_root.join(map_name),
		)?;
		definition.name = manifest_map
			.as_ref()
			.map(|map| map.api_name.clone())
			.unwrap_or_else(|| map_name.to_string());
		definition.variants.insert(String::new(), variant);
	}

	Ok(catalog)
}

fn build_variant(
	map_name: &str,
	data: &TsunkitMapResult,
	node_summary: Option<TsunkitNodeSummaryResponse>,
	enemy_root: &Path,
) -> Result<MapVariantDefinition, ParseError> {
	let rendered_colors = data
		.map_set
		.map
		.spots
		.iter()
		.filter_map(|spot| spot.no.map(|no| (no, spot.color)))
		.collect::<BTreeMap<_, _>>();
	let coord_to_cell_no = data
		.map_set
		.map
		.spots
		.iter()
		.filter_map(|spot| spot.no.map(|no| ((spot.x, spot.y), no)))
		.collect::<BTreeMap<_, _>>();
	let start_nodes = data
		.spots
		.iter()
		.filter_map(|(key, (_, _, label))| (label.as_deref() == Some("Start")).then(|| key.clone()))
		.collect::<BTreeSet<_>>();
	let mut node_to_cell_no = BTreeMap::new();

	for (node_key, (x, y, label)) in &data.spots {
		if label.as_deref() == Some("Start") {
			continue;
		}
		if let Some(cell_no) = coord_to_cell_no.get(&(*x, *y)) {
			node_to_cell_no.insert(node_key.clone(), *cell_no);
		}
	}

	let mut route_targets = BTreeMap::<String, BTreeSet<i64>>::new();
	let mut start_targets = BTreeSet::new();
	for (from, to, _, _) in data.route.values() {
		let Some(&to_cell_no) = node_to_cell_no.get(to) else {
			continue;
		};
		match from.as_deref() {
			None => {
				start_targets.insert(to_cell_no);
			}
			Some(from_key) if start_nodes.contains(from_key) => {
				start_targets.insert(to_cell_no);
			}
			Some(from_key) => {
				route_targets.entry(from_key.to_string()).or_default().insert(to_cell_no);
			}
		}
	}

	let mut battle_cell_nos = node_summary
		.map(|summary| {
			summary
				.result
				.into_iter()
				.filter_map(|(cell_no, entry)| {
					(entry.battles > 0).then(|| cell_no.parse::<i64>().ok()).flatten()
				})
				.collect::<BTreeSet<_>>()
		})
		.unwrap_or_default();
	for enemy in &data.map_set.map.enemies {
		battle_cell_nos.insert(enemy.no);
	}

	let mut enemy_fleets = BTreeMap::new();
	for (node_key, cell_no) in &node_to_cell_no {
		let path = enemy_root.join(format!("{node_key}.json"));
		let Some(response) = load_optional_json::<TsunkitEnemyCompsResponse>(&path)? else {
			continue;
		};

		let mut formations = BTreeSet::new();
		let compositions = response
			.result
			.entries
			.into_iter()
			.enumerate()
			.filter_map(|(idx, entry)| {
				let ship_ids = entry
					.main_fleet
					.into_iter()
					.chain(entry.escort_fleet)
					.map(|ship| ship.id)
					.filter(|ship_id| *ship_id > 0)
					.collect::<Vec<_>>();
				if ship_ids.is_empty() {
					return None;
				}
				if entry.formation > 0 {
					formations.insert(entry.formation);
				}
				let comp_suffix = if entry.master_id > 0 {
					entry.master_id.to_string()
				} else if !entry.node.is_empty() {
					format!("{}:{idx}", entry.node)
				} else {
					idx.to_string()
				};

				Some(EnemyComposition {
					comp_id: format!("{map_name}:{node_key}:{comp_suffix}"),
					weight: entry.count.max(1),
					ship_ids,
				})
			})
			.collect::<Vec<_>>();
		if compositions.is_empty() {
			continue;
		}

		battle_cell_nos.insert(*cell_no);
		enemy_fleets.insert(
			*cell_no,
			EnemyFleetDefinition {
				cell_no: *cell_no,
				battle_kind: 1,
				formations: if formations.is_empty() {
					vec![1]
				} else {
					formations.into_iter().collect()
				},
				compositions,
			},
		);
	}

	let airraid_cell_nos = data
		.map_set
		.map
		.airraids
		.iter()
		.filter_map(|raid| (raid.no > 0).then_some(raid.no))
		.collect::<BTreeSet<_>>();
	let boss_cell_nos = data
		.map_set
		.map
		.spots
		.iter()
		.filter_map(|spot| (spot.color == Some(-2)).then_some(spot.no).flatten())
		.collect::<BTreeSet<_>>();
	let boss_cell_no = boss_cell_nos
		.iter()
		.copied()
		.max()
		.or_else(|| battle_cell_nos.iter().copied().max())
		.or_else(|| node_to_cell_no.values().copied().max())
		.unwrap_or(1);

	let mut cells = Vec::with_capacity(node_to_cell_no.len() + 1);
	cells.push(MapCellDefinition {
		cell_no: 0,
		color_no: 0,
		event_id: 0,
		event_kind: 0,
		next_cells: start_targets.into_iter().collect(),
	});

	let mut ordered_nodes = node_to_cell_no
		.into_iter()
		.map(|(node_key, cell_no)| (cell_no, node_key))
		.collect::<Vec<_>>();
	ordered_nodes.sort_by_key(|(cell_no, _)| *cell_no);

	for (cell_no, node_key) in ordered_nodes {
		let next_cells = route_targets
			.get(&node_key)
			.map(|targets| targets.iter().copied().collect::<Vec<_>>())
			.unwrap_or_default();
		let is_battle = battle_cell_nos.contains(&cell_no) || enemy_fleets.contains_key(&cell_no);
		let is_boss = boss_cell_nos.contains(&cell_no) || (is_battle && cell_no == boss_cell_no);
		let (color_no, event_id, event_kind) = if is_boss {
			(5, 5, 1)
		} else if is_battle {
			(4, 4, 1)
		} else if airraid_cell_nos.contains(&cell_no) {
			(6, 1, 0)
		} else {
			(tsunkit_color_no(rendered_colors.get(&cell_no).copied().flatten()), 1, 0)
		};

		cells.push(MapCellDefinition {
			cell_no,
			color_no,
			event_id,
			event_kind,
			next_cells,
		});
	}

	Ok(MapVariantDefinition {
		variant_key: String::new(),
		boss_cell_no,
		cells,
		enemy_fleets,
	})
}

fn parse_map_name(map_name: &str) -> Option<(i64, i64)> {
	let (maparea_id, mapinfo_no) = map_name.split_once('-')?;
	Some((maparea_id.parse().ok()?, mapinfo_no.parse().ok()?))
}

fn tsunkit_color_no(color: Option<i64>) -> i64 {
	match color {
		Some(-2) => 5,
		Some(-3) => 0,
		Some(value) if value > 0 => value,
		_ => 6,
	}
}

fn load_json<T>(path: impl AsRef<Path>) -> Result<T, ParseError>
where
	T: serde::de::DeserializeOwned,
{
	let path = path.as_ref();
	let raw = fs::read_to_string(path).map_err(|source| ParseError::io_at(path, source))?;
	serde_json::from_str(&raw).map_err(|source| ParseError::json_at(path, source))
}

fn load_optional_json<T>(path: impl AsRef<Path>) -> Result<Option<T>, ParseError>
where
	T: serde::de::DeserializeOwned,
{
	let path = path.as_ref();
	if !path.exists() {
		return Ok(None);
	}
	load_json(path).map(Some)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_restores_graph_and_enemy_comps() {
		let fixture_root = std::env::temp_dir().join(format!(
			"emukc-tsunkit-{}",
			std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
		));
		let maps_dir = fixture_root.join("maps");
		let enemy_dir = fixture_root.join("enemycomps/1-1");
		std::fs::create_dir_all(&maps_dir).unwrap();
		std::fs::create_dir_all(&enemy_dir).unwrap();

		std::fs::write(
			maps_dir.join("1-1.json"),
			r#"{
  "result": {
    "route": {
      "0": [null, "1", 0, 0],
      "1": ["1", "A", 4, 4],
      "2": ["A", "B", 4, 4],
      "3": ["A", "C", 5, 5]
    },
    "spots": {
      "1": [260, 246, "Start"],
      "A": [597, 328, null],
      "B": [840, 204, null],
      "C": [858, 486, null]
    },
    "mapSet": {
      "map": {
        "spots": [
          { "x": 260, "y": 246 },
          { "no": 1, "x": 597, "y": 328 },
          { "no": 2, "x": 840, "y": 204 },
          { "no": 3, "x": 858, "y": 486 }
        ],
        "enemies": [],
        "airraids": []
      }
    }
  }
}"#,
		)
		.unwrap();
		std::fs::write(
			enemy_dir.join("B.json"),
			r#"{
  "result": {
    "entries": [
      {
        "node": "B",
        "mainFleet": [
          { "id": 1501 },
          { "id": 1501 }
        ],
        "escortFleet": [],
        "formation": 1,
        "count": 10,
        "masterId": 1002
      }
    ]
  }
}"#,
		)
		.unwrap();
		std::fs::write(
			enemy_dir.join("C.json"),
			r#"{
  "result": {
    "entries": [
      {
        "node": "C",
        "mainFleet": [
          { "id": 1503 },
          { "id": 1502 }
        ],
        "escortFleet": [],
        "formation": 2,
        "count": 7,
        "masterId": 1003
      }
    ]
  }
}"#,
		)
		.unwrap();

		let catalog = parse(&fixture_root, &ApiManifest::default()).unwrap();
		let definition = catalog.map_definition(11).unwrap();
		let variant = definition.variant("").unwrap();
		let first = variant.first_progress_cell_no().unwrap();
		let branch = variant.cell(1).unwrap();
		let boss = variant.cell(3).unwrap();
		let node_b = variant.enemy_fleets.get(&2).unwrap();
		let node_c = variant.enemy_fleets.get(&3).unwrap();

		assert_eq!(first, 1);
		assert_eq!(branch.next_cells, vec![2, 3]);
		assert_eq!(variant.boss_cell_no, 3);
		assert_eq!(boss.event_kind, 1);
		assert_eq!(node_b.compositions[0].ship_ids, vec![1501, 1501]);
		assert_eq!(node_b.formations, vec![1]);
		assert_eq!(node_c.compositions[0].ship_ids, vec![1503, 1502]);
		assert_eq!(node_c.formations, vec![2]);

		std::fs::remove_dir_all(fixture_root).unwrap();
	}
}
