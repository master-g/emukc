use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

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
