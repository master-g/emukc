use std::{collections::BTreeMap, fs, path::Path};

use emukc_model::{
    codex::map::{
        MapCatalog, MapCellDefinition, MapDefinition, MapVariantDefinition, extract_max_hp,
        split_map_id,
    },
    kc2::start2::ApiManifest,
};
use serde::Deserialize;
use serde_yaml::Deserializer;

use crate::parser::error::ParseError;

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

/// Boss cell appearance: red node, boss battle event.
const BOSS_CELL: (i64, i64, i64) = (5, 5, 1);

/// Regular battle cell appearance: orange node, normal battle event.
const BATTLE_CELL: (i64, i64, i64) = (4, 4, 1);

/// Empty/resource cell appearance: blue node, non-battle event.
const EMPTY_CELL: (i64, i64, i64) = (6, 1, 0);

pub(super) fn load_map_catalog_from_kcdata_root(
    kcdata_root: impl AsRef<Path>,
    manifest: &ApiManifest,
) -> Result<(MapCatalog, usize), ParseError> {
    let mut catalog = MapCatalog::from_manifest(manifest);
    let mut kcdata_parse_errors: usize = 0;
    let map_root = kcdata_root.as_ref().join("_map");
    let Ok(entries) = fs::read_dir(&map_root) else {
        return Ok((catalog, kcdata_parse_errors));
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
                Err(e) => {
                    let path_display = path.display().to_string();
                    tracing::warn!(%path_display, ?e, "skip malformed kcdata yaml");
                    kcdata_parse_errors += 1;
                    continue;
                }
            }
        }
        let Some(parsed) = parsed else {
            continue;
        };

        let (maparea_id, _mapinfo_no) = split_map_id(parsed.id);
        let manifest_map =
            manifest.api_mst_mapinfo.iter().find(|map| map.api_id == parsed.id).cloned();
        let entry = catalog.maps.entry(parsed.id).or_insert_with(|| {
            let mut def = MapDefinition::minimal(parsed.id);
            def.name = parsed.name.clone();
            def.level = manifest_map.as_ref().map(|map| map.api_level).unwrap_or(1);
            def.sally_flag =
                manifest_map.as_ref().map(|map| map.api_sally_flag.clone()).unwrap_or_default();
            def.is_event = maparea_id > 7;
            def.required_defeat_count =
                manifest_map.as_ref().and_then(|map| map.api_required_defeat_count);
            def.max_hp = manifest_map.as_ref().and_then(extract_max_hp);
            def
        });

        entry.name = parsed.name.clone();
        entry.variants.insert(String::new(), build_variant_from_kcdata(&parsed));
    }

    Ok((catalog, kcdata_parse_errors))
}

fn build_variant_from_kcdata(data: &KcDataMapData) -> MapVariantDefinition {
    // Pre-compute: for each node, which routes depart from it?
    let mut routes_from_node: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for (&route_id, route) in &data.routes {
        if let Some(from_key) = route.from.as_ref().and_then(route_node_key) {
            routes_from_node.entry(from_key).or_default().push(route_id);
        }
    }

    let mut cells = Vec::with_capacity(data.routes.len());
    let mut boss_cell_no: i64 = 0;

    for (&route_id, route) in &data.routes {
        let target_key = route_node_key(&route.to);
        let is_start = route.from.is_none();

        let cell_meta = target_key.as_ref().and_then(|key| data.cells.get(key));
        let has_battle = cell_meta.is_some_and(|c| c.boss || !c.name.trim().is_empty());
        let is_boss = cell_meta.is_some_and(|c| c.boss);

        let (color_no, event_id, event_kind) = if is_boss {
            boss_cell_no = route_id;
            BOSS_CELL
        } else if has_battle {
            BATTLE_CELL
        } else {
            EMPTY_CELL
        };

        // next_cells = route IDs departing from this route's target node
        let next_cells = target_key
            .as_ref()
            .and_then(|key| routes_from_node.get(key))
            .cloned()
            .unwrap_or_default();

        let node_label = if is_start {
            Some("Start".to_string())
        } else {
            target_key.clone()
        };

        cells.push(MapCellDefinition {
            cell_no: route_id,
            color_no,
            event_id,
            event_kind,
            next_cells,
            node_label,
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

fn route_node_key(node: &KcDataNode) -> Option<String> {
    match node {
        KcDataNode::Int(value) => Some(value.to_string()),
        KcDataNode::String(value) => Some(value.clone()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

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
    fn build_variant_from_kcdata_produces_route_based_cells() {
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
                data.routes.len(),
                "{} produced unexpected cell count (expected {} routes, got {} cells)",
                path.display(),
                data.routes.len(),
                variant.cells.len()
            );

            // Verify cell_nos match route IDs exactly
            let route_ids = data.routes.keys().copied().collect::<BTreeSet<_>>();
            assert_eq!(cell_nos, route_ids, "{} cell_nos don't match route IDs", path.display());

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
    fn build_variant_from_all_repo_kcdata_maps_routes_target_numeric_nodes_get_metadata() {
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

            // For each route targeting a numeric node with metadata in data.cells,
            // verify the cell picks up that metadata
            for (&route_id, route) in &data.routes {
                let Some(target_key) = route_node_key(&route.to) else {
                    continue;
                };
                let Some(cell_meta) = data.cells.get(&target_key) else {
                    continue;
                };
                if target_key.parse::<i64>().is_err() {
                    continue;
                }
                let cell = variant.cells.iter().find(|c| c.cell_no == route_id).unwrap();
                if cell_meta.boss {
                    assert_eq!(
                        cell.color_no,
                        5,
                        "{} route {} targets boss node {} but color_no != 5",
                        path.display(),
                        route_id,
                        target_key
                    );
                }
            }
        }
    }

    // ------------------------------------------------------------------ parse-error surfacing

    /// A corrupt YAML file increments the parse-error counter while valid siblings are still loaded.
    #[test]
    fn corrupt_kcdata_yaml_increments_error_counter_and_loads_valid_siblings() {
        let dir = tempfile::tempdir().unwrap();
        let map_root = dir.path().join("kc_data/_map");
        std::fs::create_dir_all(&map_root).unwrap();

        // One valid YAML file.
        let valid_yaml = r#"---
layout: json
order: 11
data:
  id: 11
  name: valid map
  routes:
    0:
      from: null
      to: A
  cells:
    A:
      name: test node
      boss: true
---"#;
        std::fs::write(map_root.join("valid.yaml"), valid_yaml).unwrap();

        // One YAML that parses structurally but fails typed deserialization (wrong types).
        let corrupt_yaml = r#"---
layout: json
order: not_a_number
data:
  id: also_not_a_number
  name: corrupt
  routes: []
  cells: []
"#;
        std::fs::write(map_root.join("corrupt.yaml"), corrupt_yaml).unwrap();

        let manifest = emukc_model::kc2::start2::ApiManifest::default();
        let (catalog, parse_errors) =
            load_map_catalog_from_kcdata_root(dir.path().join("kc_data"), &manifest).unwrap();

        assert_eq!(parse_errors, 1, "one corrupt file must increment the counter to 1");
        assert_eq!(catalog.maps.len(), 1, "valid map must still be loaded");
        assert!(catalog.maps.contains_key(&11), "map 11 must be present");
    }
}
