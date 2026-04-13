use std::collections::{BTreeMap, BTreeSet};

use super::{
    EnemyFleetDefinition, MapCellDefinition, MapDefinition, MapVariantDefinition, RouteRule,
};

pub(super) fn merge_definition(definition: &mut MapDefinition, other: MapDefinition) {
    if definition.name.is_empty() {
        definition.name = other.name;
    }
    if definition.level <= 0 {
        definition.level = other.level;
    }
    if definition.sally_flag.is_empty() {
        definition.sally_flag = other.sally_flag;
    }
    if !definition.is_event {
        definition.is_event = other.is_event;
    }
    if definition.airbase_count.is_none() {
        definition.airbase_count = other.airbase_count;
    }
    if definition.gauge_type.is_none() {
        definition.gauge_type = other.gauge_type;
    }
    if definition.gauge_count.is_none() {
        definition.gauge_count = other.gauge_count;
    }
    if definition.required_defeat_count.is_none() {
        definition.required_defeat_count = other.required_defeat_count;
    }
    if definition.max_hp.is_none() {
        definition.max_hp = other.max_hp;
    }
    if definition.default_variant.is_empty() {
        definition.default_variant = other.default_variant;
    }
    if definition.rank_stage_ids.is_empty() {
        definition.rank_stage_ids = other.rank_stage_ids;
    }
    let definition_has_named_variants = definition.variants.keys().any(|key| !key.is_empty());
    let fallback_variant = other.variants.get("").cloned();
    for (variant_key, variant) in other.variants {
        if variant_key.is_empty() && definition_has_named_variants {
            continue;
        }
        match definition.variants.entry(variant_key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(variant);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                merge_variant_definition(entry.get_mut(), variant);
            }
        }
    }
    if let Some(fallback_variant) = fallback_variant {
        for (variant_key, variant) in &mut definition.variants {
            if variant_key.is_empty() {
                continue;
            }
            merge_variant_definition(variant, fallback_variant.clone());
        }
    }
}

fn merge_variant_definition(definition: &mut MapVariantDefinition, other: MapVariantDefinition) {
    let other = remap_variant_to_definition_identity(definition, other);
    let had_inferred_start = definition.parse_warnings.iter().any(|warning| {
        warning == "missing_start_routes" || warning.starts_with("inferred_multi_root_start")
    });
    let other_start =
        other.cells.iter().find(|cell| cell.cell_no == 0).map(|cell| cell.next_cells.clone());
    if definition.boss_cell_no <= 0 {
        definition.boss_cell_no = other.boss_cell_no;
    }
    if definition.required_defeat_count.is_none() {
        definition.required_defeat_count = other.required_defeat_count;
    }
    if definition.clear_to_variant_key.is_none() {
        definition.clear_to_variant_key = other.clear_to_variant_key;
    }
    merge_cells(&mut definition.cells, other.cells);
    for (from_cell_no, rules) in other.routing_rules {
        definition.routing_rules.entry(from_cell_no).or_insert(rules);
    }
    for (cell_no, fleet) in other.enemy_fleets {
        definition.enemy_fleets.entry(cell_no).or_insert(fleet);
    }
    for (cell_no, drops) in other.ship_drops {
        definition.ship_drops.entry(cell_no).or_insert(drops);
    }
    if had_inferred_start
        && let Some(other_start) = other_start
        && !other_start.is_empty()
        && let Some(start_cell) = definition.cells.iter_mut().find(|cell| cell.cell_no == 0)
    {
        start_cell.next_cells = other_start;
        definition.parse_warnings.retain(|warning| {
            warning != "missing_start_routes" && !warning.starts_with("inferred_multi_root_start")
        });
        if !definition.parse_warnings.iter().any(|warning| warning == "structural_start_fallback") {
            definition.parse_warnings.push("structural_start_fallback".to_string());
        }
    }
    if definition.parse_warnings.is_empty() {
        definition.parse_warnings = other.parse_warnings;
    }
}

fn remap_variant_to_definition_identity(
    definition: &MapVariantDefinition,
    mut other: MapVariantDefinition,
) -> MapVariantDefinition {
    let cell_no_map = semantic_cell_no_map(definition, &other);
    if cell_no_map.is_empty() {
        return other;
    }

    // Preserve the primary variant's numbering, but let secondary sources join on
    // stable node labels when both sides expose a unique semantic label.
    other.boss_cell_no = remap_cell_no(other.boss_cell_no, &cell_no_map);
    for cell in &mut other.cells {
        cell.cell_no = remap_cell_no(cell.cell_no, &cell_no_map);
        remap_cell_nos(&mut cell.next_cells, &cell_no_map);
    }

    let mut routing_rules = BTreeMap::<i64, Vec<RouteRule>>::new();
    for (from_cell_no, rules) in other.routing_rules {
        let mapped_from = remap_cell_no(from_cell_no, &cell_no_map);
        routing_rules.entry(mapped_from).or_default().extend(rules.into_iter().map(|mut rule| {
            rule.from_cell_no = remap_cell_no(rule.from_cell_no, &cell_no_map);
            rule.to_cell_no = remap_cell_no(rule.to_cell_no, &cell_no_map);
            rule
        }));
    }
    other.routing_rules = routing_rules;

    let mut enemy_fleets = BTreeMap::<i64, EnemyFleetDefinition>::new();
    for (cell_no, mut fleet) in other.enemy_fleets {
        let mapped_cell_no = remap_cell_no(cell_no, &cell_no_map);
        fleet.cell_no = remap_cell_no(fleet.cell_no, &cell_no_map);
        enemy_fleets.insert(mapped_cell_no, fleet);
    }
    other.enemy_fleets = enemy_fleets;

    other.ship_drops = other
        .ship_drops
        .into_iter()
        .map(|(cell_no, drops)| (remap_cell_no(cell_no, &cell_no_map), drops))
        .collect();

    other
}

fn semantic_cell_no_map(
    definition: &MapVariantDefinition,
    other: &MapVariantDefinition,
) -> BTreeMap<i64, i64> {
    let definition_labels = unique_labeled_cells(&definition.cells);
    let other_labels = unique_labeled_cells(&other.cells);
    other_labels
        .into_iter()
        .filter_map(|(label, other_cell_no)| {
            definition_labels
                .get(&label)
                .copied()
                .map(|definition_cell_no| (other_cell_no, definition_cell_no))
        })
        .collect()
}

fn unique_labeled_cells(cells: &[MapCellDefinition]) -> BTreeMap<String, i64> {
    let mut labels = BTreeMap::new();
    let mut duplicates = BTreeSet::new();

    for cell in cells {
        let Some(label) = cell.node_label.as_ref().filter(|label| !label.is_empty()) else {
            continue;
        };
        if duplicates.contains(label) {
            continue;
        }
        if let Some(previous) = labels.insert(label.clone(), cell.cell_no)
            && previous != cell.cell_no
        {
            labels.remove(label);
            duplicates.insert(label.clone());
        }
    }

    labels
}

fn remap_cell_nos(cell_nos: &mut Vec<i64>, cell_no_map: &BTreeMap<i64, i64>) {
    let mut remapped = Vec::with_capacity(cell_nos.len());
    for cell_no in std::mem::take(cell_nos) {
        let mapped = remap_cell_no(cell_no, cell_no_map);
        if !remapped.contains(&mapped) {
            remapped.push(mapped);
        }
    }
    *cell_nos = remapped;
}

fn remap_cell_no(cell_no: i64, cell_no_map: &BTreeMap<i64, i64>) -> i64 {
    cell_no_map.get(&cell_no).copied().unwrap_or(cell_no)
}

fn merge_cells(cells: &mut Vec<MapCellDefinition>, other_cells: Vec<MapCellDefinition>) {
    let mut merged = cells.drain(..).map(|cell| (cell.cell_no, cell)).collect::<BTreeMap<_, _>>();

    for other in other_cells {
        match merged.entry(other.cell_no) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(other);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let cell = entry.get_mut();
                if cell.color_no <= 0 && other.color_no > 0 {
                    cell.color_no = other.color_no;
                }
                if cell.event_id <= 0 && other.event_id > 0 {
                    cell.event_id = other.event_id;
                }
                if cell.event_kind <= 0 && other.event_kind > 0 {
                    cell.event_kind = other.event_kind;
                }
                if cell.next_cells.is_empty() && !other.next_cells.is_empty() {
                    cell.next_cells = other.next_cells;
                }
                if cell.node_label.is_none() {
                    cell.node_label = other.node_label;
                }
                if cell.master_cell_id.is_none() {
                    cell.master_cell_id = other.master_cell_id;
                }
                if cell.distance.is_none() {
                    cell.distance = other.distance;
                }
            }
        }
    }

    *cells = merged.into_values().collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex::map::{RoutePredicate, ShipDropDefinition};

    fn cell(
        cell_no: i64,
        node_label: &str,
        next_cells: Vec<i64>,
        event_id: i64,
        event_kind: i64,
        color_no: i64,
    ) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            color_no,
            event_id,
            event_kind,
            next_cells,
            node_label: Some(node_label.to_string()),
            master_cell_id: None,
            distance: None,
        }
    }

    #[test]
    fn merge_variant_definition_remaps_secondary_cells_by_node_label() {
        let mut definition = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 0,
            cells: vec![
                cell(0, "Start", vec![], 0, 0, 0),
                cell(1, "A", vec![], 0, 0, 0),
                cell(2, "B", vec![], 0, 0, 0),
                cell(3, "C", vec![], 0, 0, 0),
            ],
            routing_rules: BTreeMap::new(),
            enemy_fleets: BTreeMap::new(),
            ship_drops: BTreeMap::new(),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };
        let other = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 1,
            cells: vec![
                cell(0, "Start", vec![2, 1], 0, 0, 0),
                cell(1, "C", vec![], 5, 1, 5),
                cell(2, "A", vec![3], 4, 1, 4),
                cell(3, "B", vec![1], 4, 1, 4),
            ],
            routing_rules: BTreeMap::from([
                (
                    2,
                    vec![RouteRule {
                        from_cell_no: 2,
                        to_cell_no: 3,
                        priority: 0,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Always,
                        raw_text: String::new(),
                    }],
                ),
                (
                    3,
                    vec![RouteRule {
                        from_cell_no: 3,
                        to_cell_no: 1,
                        priority: 1,
                        weight: None,
                        probability_pct: None,
                        predicate: RoutePredicate::Always,
                        raw_text: String::new(),
                    }],
                ),
            ]),
            enemy_fleets: BTreeMap::from([(
                1,
                EnemyFleetDefinition {
                    cell_no: 1,
                    battle_kind: 1,
                    formations: vec![1],
                    compositions: Vec::new(),
                },
            )]),
            ship_drops: BTreeMap::from([(1, vec![ShipDropDefinition::default()])]),
            required_defeat_count: None,
            clear_to_variant_key: None,
            parse_warnings: Vec::new(),
        };

        merge_variant_definition(&mut definition, other);

        assert_eq!(definition.cell(0).unwrap().next_cells, vec![1, 3]);
        assert_eq!(definition.cell(1).unwrap().next_cells, vec![2]);
        assert_eq!(definition.cell(2).unwrap().next_cells, vec![3]);
        assert_eq!(definition.cell(3).unwrap().event_id, 5);
        assert_eq!(definition.boss_cell_no, 3);
        assert_eq!(definition.routing_rules.get(&1).unwrap()[0].to_cell_no, 2);
        assert_eq!(definition.routing_rules.get(&2).unwrap()[0].to_cell_no, 3);
        assert!(definition.enemy_fleets.contains_key(&3));
        assert!(definition.ship_drops.contains_key(&3));
    }
}
