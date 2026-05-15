use std::{collections::BTreeMap, fs, path::Path, sync::LazyLock};

use emukc_model::{
    codex::map::{
        EnemyComposition, EnemyFleetDefinition, MapCatalog, MapCellDefinition, MapDefinition,
        MapResetPolicy, MapVariantDefinition, RoutePredicate, RouteRule, ShipDropDefinition,
    },
    kc2::start2::ApiManifest,
};
use regex::Regex;
use scraper::{Html, Selector};

use super::error::ParseError;
use crate::wikiwiki_map_download::wikiwiki_map_page_url;
mod drop;
mod enemy;
mod html;
mod resolver;
mod route;
mod types;
use drop::*;
use enemy::*;
use html::*;
use resolver::*;
use route::*;
use types::*;
pub use types::{
    EnemyNodeRows, RouteRuleDraft, ShipDropDraft, WikiwikiLabelOverlay, WikiwikiMapOverlayCatalog,
    WikiwikiMapOverlayDefinition,
};

static SELECTOR_TABLE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("table").expect("valid table selector"));
static SELECTOR_FOLD_CONTAINER: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.fold-container").expect("valid fold container selector"));
static SELECTOR_ROW: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("tr").expect("valid row selector"));
static SELECTOR_CELL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("th, td").expect("valid cell selector"));
static RE_NODE_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"([A-Z][A-Z0-9]?)").expect("valid node label regex"));
static RE_MULTIPLIER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<name>.+?)[x×](?P<count>\d+)$").expect("valid ship multiplier regex")
});
static RE_PAREN_ANNOTATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[（(][^()（）]*[)）]").expect("valid parenthetical annotation regex")
});
static RE_SAME_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<pattern>パターン\d+)と同(?:じ|編成)$").expect("valid same-pattern regex")
});
static RE_FOOTNOTE_MARKER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[*＊]\d+").expect("valid footnote marker regex"));
static RE_SHIP_TYPE_COUNT_CLAUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<name>.+?)(?P<count>\d+)(?:隻)?(?P<op>以上|以下|ちょうど|過不足なく)?$")
        .expect("valid ship type count clause regex")
});

/// Parse a cached `wikiwiki_map` directory into a compact runtime [`MapCatalog`].
pub fn parse(root: impl AsRef<Path>, manifest: &ApiManifest) -> Result<MapCatalog, ParseError> {
    parse_debug(root, manifest).map(|catalog| catalog.into_map_catalog(manifest))
}

/// Parse a cached `wikiwiki_map` directory into a debug-oriented source model.
pub fn parse_debug(
    root: impl AsRef<Path>,
    manifest: &ApiManifest,
) -> Result<WikiwikiMapCatalog, ParseError> {
    let pages_root = root.as_ref().join("pages");
    let entries =
        fs::read_dir(&pages_root).map_err(|source| ParseError::io_at(&pages_root, source))?;
    let ship_types = ShipTypeResolver::new(manifest);
    let ships = ShipResolver::new(manifest);
    let mut catalog = WikiwikiMapCatalog::default();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("html") {
            continue;
        }

        let Some(map_name) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some((maparea_id, mapinfo_no)) = parse_map_name(map_name) else {
            continue;
        };
        let raw = fs::read_to_string(&path).map_err(|source| ParseError::io_at(&path, source))?;
        let map_id = maparea_id * 10 + mapinfo_no;
        let definition = parse_map_page(
            map_name,
            map_id,
            maparea_id,
            mapinfo_no,
            &raw,
            manifest,
            &ship_types,
            &ships,
        )?;
        catalog.maps.insert(map_id, definition);
    }

    Ok(catalog)
}

impl WikiwikiMapCatalog {
    /// Convert the extractor output into the runtime `MapCatalog` model.
    pub fn into_map_catalog(self, manifest: &ApiManifest) -> MapCatalog {
        self.into_map_catalog_with_overlay(manifest).0
    }

    /// Convert into runtime `MapCatalog` and extract label-keyed overlay catalog.
    pub fn into_map_catalog_with_overlay(
        self,
        manifest: &ApiManifest,
    ) -> (MapCatalog, WikiwikiMapOverlayCatalog) {
        let mut catalog = MapCatalog::default();
        let mut overlay_catalog = WikiwikiMapOverlayCatalog::default();
        for (map_id, definition) in self.maps {
            let manifest_map = manifest.api_mst_mapinfo.iter().find(|map| map.api_id == map_id);
            let area_type = manifest
                .api_mst_maparea
                .iter()
                .find(|area| area.api_id == definition.maparea_id)
                .map(|area| area.api_type)
                .unwrap_or_default();
            let default_variant = definition.default_variant.clone();
            let first_variant_required_defeat_count = definition
                .variants
                .get(&default_variant)
                .and_then(|variant| variant.required_defeat_count)
                .or_else(|| {
                    definition
                        .variants
                        .values()
                        .next()
                        .and_then(|variant| variant.required_defeat_count)
                });
            let gauge_count =
                (definition.variants.len() > 1).then_some(definition.variants.len() as i64);
            let variants = definition
                .variants
                .into_iter()
                .map(|(variant_key, variant)| {
                    let mut node_labels_to_cell = variant
                        .nodes
                        .iter()
                        .map(|node| (node.label.clone(), node.cell_no))
                        .collect::<BTreeMap<_, _>>();
                    node_labels_to_cell.insert(ENTRY_NODE_LABEL.to_string(), 0);
                    let mut routing_rules = BTreeMap::<i64, Vec<RouteRule>>::new();
                    for rule in &variant.routing_rules {
                        let mut rule = rule.clone();
                        rule.predicate =
                            rewrite_route_predicate_labels(rule.predicate, &node_labels_to_cell);
                        routing_rules.entry(rule.from_cell_no).or_default().push(rule);
                    }
                    for rules in routing_rules.values_mut() {
                        rules.sort_by_key(|rule| rule.priority);
                    }

                    let inferred_root_targets = variant
                        .nodes
                        .iter()
                        .filter(|node| {
                            !variant.routing_rules.iter().any(|rule| {
                                rule.to_cell_no == node.cell_no && rule.from_cell_no != node.cell_no
                            })
                        })
                        .map(|node| node.cell_no)
                        .collect::<Vec<_>>();
                    let mut parse_warnings = variant.parse_warnings;
                    let start_targets = routing_rules
                        .get(&0)
                        .map(|rules| ordered_route_targets(rules))
                        .filter(|targets| !targets.is_empty())
                        .unwrap_or_else(|| {
                            if inferred_root_targets.len() > 1 {
                                parse_warnings.push(format!(
                                    "inferred_multi_root_start:{}",
                                    inferred_root_targets
                                        .iter()
                                        .map(i64::to_string)
                                        .collect::<Vec<_>>()
                                        .join(",")
                                ));
                            }
                            if inferred_root_targets.is_empty() && !variant.nodes.is_empty() {
                                parse_warnings.push("missing_start_routes".to_string());
                            }
                            if inferred_root_targets.is_empty() {
                                variant
                                    .nodes
                                    .first()
                                    .map(|node| vec![node.cell_no])
                                    .unwrap_or_default()
                            } else {
                                inferred_root_targets
                            }
                        });

                    let mut cells = Vec::with_capacity(variant.nodes.len() + 1);
                    cells.push(MapCellDefinition {
                        cell_no: 0,
                        color_no: 0,
                        event_id: 0,
                        event_kind: 0,
                        next_cells: start_targets,
                        node_label: Some(ENTRY_NODE_LABEL.to_string()),
                        master_cell_id: None,
                        distance: None,
                    });

                    let boss_cell_no = variant
                        .nodes
                        .iter()
                        .find(|node| node.is_boss)
                        .map(|node| node.cell_no)
                        .or_else(|| {
                            variant
                                .nodes
                                .iter()
                                .filter(|node| node.is_battle)
                                .map(|node| node.cell_no)
                                .max()
                        })
                        .unwrap_or(1);

                    for node in &variant.nodes {
                        let (color_no, event_id, event_kind) = if node.is_boss {
                            (5, 5, 1)
                        } else if node.is_battle {
                            (4, 4, 1)
                        } else {
                            (6, 1, 0)
                        };
                        cells.push(MapCellDefinition {
                            cell_no: node.cell_no,
                            color_no,
                            event_id,
                            event_kind,
                            next_cells: vec![],
                            node_label: Some(node.label.clone()),
                            master_cell_id: None,
                            distance: None,
                        });
                    }

                    let enemy_fleets = variant
                        .enemy_fleets
                        .into_iter()
                        .map(|fleet| {
                            (
                                fleet.cell_no,
                                EnemyFleetDefinition {
                                    cell_no: fleet.cell_no,
                                    battle_kind: fleet.battle_kind,
                                    formations: fleet.formations,
                                    compositions: fleet
                                        .compositions
                                        .into_iter()
                                        .map(compact_enemy_composition)
                                        .collect(),
                                },
                            )
                        })
                        .collect::<BTreeMap<_, _>>();

                    (
                        variant_key.clone(),
                        MapVariantDefinition {
                            variant_key,
                            boss_cell_no,
                            cells,
                            routing_rules,
                            enemy_fleets,
                            ship_drops: variant.ship_drops,
                            required_defeat_count: variant.required_defeat_count,
                            clear_to_variant_key: variant.clear_to_variant_key,
                            parse_warnings,
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            catalog.maps.insert(
                map_id,
                MapDefinition {
                    map_id,
                    maparea_id: definition.maparea_id,
                    mapinfo_no: definition.mapinfo_no,
                    name: manifest_map.map(|map| map.api_name.clone()).unwrap_or(definition.name),
                    level: manifest_map.map(|map| map.api_level).unwrap_or(1),
                    sally_flag: manifest_map
                        .map(|map| map.api_sally_flag.clone())
                        .unwrap_or_default(),
                    is_event: area_type == 1,
                    reset_policy: MapResetPolicy::Never,
                    airbase_count: None,
                    gauge_type: None,
                    gauge_count,
                    required_defeat_count: first_variant_required_defeat_count
                        .or_else(|| manifest_map.and_then(|map| map.api_required_defeat_count)),
                    max_hp: None,
                    default_variant,
                    rank_stage_ids: BTreeMap::new(),
                    variants,
                },
            );

            if !definition.overlays.is_empty() {
                overlay_catalog.maps.insert(
                    map_id,
                    WikiwikiMapOverlayDefinition {
                        map_id,
                        variants: definition.overlays,
                    },
                );
            }
        }

        (catalog, overlay_catalog)
    }

    pub fn to_debug_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| serde_json::json!({}))
    }
}

#[expect(clippy::too_many_arguments)]
fn parse_map_page(
    map_name: &str,
    map_id: i64,
    maparea_id: i64,
    mapinfo_no: i64,
    raw_html: &str,
    manifest: &ApiManifest,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Result<WikiwikiMapDefinition, ParseError> {
    let document = Html::parse_document(raw_html);
    let route_sections = find_route_table_sections(&document);
    if route_sections.is_empty() {
        return Err(ParseError::Generic(format!("route table not found for {map_name}")));
    }
    let enemy_table =
        find_table_by_headers(&document, &["出現場所", "パターン", "EXP", "出現艦船"])
            .ok_or_else(|| ParseError::Generic(format!("enemy table not found for {map_name}")))?;
    let drop_table = find_drop_table(&document);
    let gauge_defeat_counts = parse_gauge_defeat_counts(&document);
    let mut base_warnings = Vec::new();
    let enemy_nodes = parse_enemy_table(map_name, &enemy_table, ships, &mut base_warnings)?;
    let drop_drafts = drop_table
        .as_ref()
        .map(|table| parse_drop_table(map_name, table, ships, &mut base_warnings))
        .unwrap_or_default();
    let variant_keys = route_sections
        .iter()
        .enumerate()
        .map(|(idx, section)| {
            route_section_variant_key(&section.summary, idx, route_sections.len())
        })
        .collect::<Vec<_>>();
    let mut variants = BTreeMap::new();
    let mut overlays = BTreeMap::new();

    for (idx, section) in route_sections.iter().enumerate() {
        let mut warnings = base_warnings.clone();
        let mut route_rules = parse_route_table(&section.rows, ship_types, ships, &mut warnings)?;
        postprocess_route_probabilities(&mut route_rules);
        check_mixed_routing_encoding(&route_rules, &mut warnings);
        let enemy_nodes = enemy_nodes.clone();

        // Capture label-keyed overlay before build_nodes() converts to cell_nos.
        let variant_key = variant_keys[idx].clone();
        overlays.insert(
            variant_key.clone(),
            WikiwikiLabelOverlay {
                variant_key: variant_key.clone(),
                routing_rules: route_rules.clone(),
                enemy_nodes: enemy_nodes.clone(),
                ship_drops: drop_drafts.clone(),
                required_defeat_count: gauge_defeat_counts.get(idx).copied(),
                parse_warnings: warnings.clone(),
            },
        );

        let nodes = build_nodes(&route_rules, &enemy_nodes);
        let node_to_cell =
            nodes.iter().map(|node| (node.label.clone(), node.cell_no)).collect::<BTreeMap<_, _>>();
        let mut node_to_cell = node_to_cell;
        node_to_cell.insert(ENTRY_NODE_LABEL.to_string(), 0);
        let routing_rules = route_rules
            .into_iter()
            .filter_map(|rule| {
                node_to_cell.get(&rule.from_label).zip(node_to_cell.get(&rule.to_label)).map(
                    |(&from_cell_no, &to_cell_no)| {
                        let predicate = rule.predicate;
                        RouteRule {
                            from_cell_no,
                            to_cell_no,
                            priority: 0,
                            weight: rule.probability_pct.map(probability_to_weight),
                            probability_pct: rule.probability_pct,
                            raw_text: compact_route_raw_text(&predicate, rule.raw_text),
                            predicate,
                        }
                    },
                )
            })
            .enumerate()
            .map(|(priority, mut rule)| {
                rule.priority = priority as i64;
                rule
            })
            .collect::<Vec<_>>();
        let enemy_fleets = enemy_nodes
            .into_iter()
            .filter_map(|(node_label, node)| {
                node_to_cell.get(&node_label).copied().map(|cell_no| WikiwikiEnemyFleetDefinition {
                    node_label,
                    cell_no,
                    battle_kind: 1,
                    formations: collect_formations(&node.compositions),
                    compositions: node.compositions,
                })
            })
            .collect::<Vec<_>>();
        let ship_drops = drop_drafts
            .iter()
            .filter_map(|draft| {
                node_to_cell
                    .get(&draft.node_label)
                    .copied()
                    .map(|cell_no| (cell_no, draft.drop.clone()))
            })
            .fold(BTreeMap::<i64, Vec<ShipDropDefinition>>::new(), |mut acc, (cell_no, drop)| {
                acc.entry(cell_no).or_default().push(drop);
                acc
            });
        variants.insert(
            variant_key.clone(),
            WikiwikiMapVariantDefinition {
                variant_key,
                nodes,
                routing_rules,
                enemy_fleets,
                ship_drops,
                required_defeat_count: gauge_defeat_counts.get(idx).copied(),
                clear_to_variant_key: None,
                parse_warnings: warnings,
            },
        );
    }
    for pair in variant_keys.windows(2) {
        let [current, next] = pair else {
            continue;
        };
        if let Some(variant) = variants.get_mut(current) {
            variant.clear_to_variant_key = Some(next.clone());
        }
    }
    let default_variant = if variants.len() == 1 && variants.contains_key("") {
        String::new()
    } else {
        variant_keys.first().cloned().unwrap_or_default()
    };

    Ok(WikiwikiMapDefinition {
        map_id,
        maparea_id,
        mapinfo_no,
        name: manifest
            .api_mst_mapinfo
            .iter()
            .find(|map| map.api_id == map_id)
            .map(|map| map.api_name.clone())
            .unwrap_or_else(|| map_name.to_string()),
        source_url: wikiwiki_map_page_url(map_name).unwrap_or_default(),
        default_variant,
        variants,
        overlays,
    })
}

fn compact_enemy_composition(mut composition: EnemyComposition) -> EnemyComposition {
    composition.raw_ship_names.clear();
    composition
}

fn ordered_route_targets(rules: &[RouteRule]) -> Vec<i64> {
    let mut seen = std::collections::BTreeSet::new();
    let mut targets = Vec::new();
    for rule in rules {
        if seen.insert(rule.to_cell_no) {
            targets.push(rule.to_cell_no);
        }
    }
    targets
}

fn rewrite_route_predicate_labels(
    predicate: RoutePredicate,
    node_labels_to_cell: &BTreeMap<String, i64>,
) -> RoutePredicate {
    match predicate {
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => RoutePredicate::VisitedNode {
            cell_nos: node_labels
                .into_iter()
                .filter_map(|label| node_labels_to_cell.get(&label).copied())
                .collect(),
            visited,
        },
        RoutePredicate::And(predicates) => RoutePredicate::And(
            predicates
                .into_iter()
                .map(|predicate| rewrite_route_predicate_labels(predicate, node_labels_to_cell))
                .collect(),
        ),
        RoutePredicate::Or(predicates) => RoutePredicate::Or(
            predicates
                .into_iter()
                .map(|predicate| rewrite_route_predicate_labels(predicate, node_labels_to_cell))
                .collect(),
        ),
        RoutePredicate::Not(predicate) => RoutePredicate::Not(Box::new(
            rewrite_route_predicate_labels(*predicate, node_labels_to_cell),
        )),
        other => other,
    }
}

fn parse_same_pattern_alias(text: &str) -> Option<String> {
    RE_SAME_PATTERN
        .captures(text)
        .and_then(|caps| caps.name("pattern"))
        .map(|value| normalize_text(value.as_str()))
}

fn normalize_text(text: &str) -> String {
    text.replace('\u{a0}', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn sanitize_route_text(text: &str) -> String {
    let text = normalize_text(text).replace('_', " ");
    let text = RE_FOOTNOTE_MARKER.replace_all(&text, "");
    let text = text.replace(['?', '？'], "");
    normalize_text(&text)
}

fn sanitize_drop_text(text: &str) -> String {
    let text = normalize_text(text);
    let text = RE_FOOTNOTE_MARKER.replace_all(&text, "");
    let text = text.trim_matches(|ch| matches!(ch, '※' | '＊' | '*')).to_string();
    normalize_text(&text)
}

fn parse_map_name(map_name: &str) -> Option<(i64, i64)> {
    let (maparea_id, mapinfo_no) = map_name.split_once('-')?;
    Some((maparea_id.parse().ok()?, mapinfo_no.parse().ok()?))
}

fn probability_to_weight(probability_pct: f64) -> i64 {
    (probability_pct * 100.0).round() as i64
}

#[cfg(test)]
mod tests;
