use std::collections::BTreeMap;

use super::{EnemyFleetDefinition, MapCellDefinition, MapDefinition, MapVariantDefinition};

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
    if definition.gauge_type_e.is_none() {
        definition.gauge_type_e = other.gauge_type_e;
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

/// Merge a secondary source's variant onto the primary one.
///
/// Routing rules are not merged here: they only enter the catalog through the
/// wikiwiki label overlay, which resolves them against the final topology.
fn merge_variant_definition(definition: &mut MapVariantDefinition, other: MapVariantDefinition) {
    let other = remap_variant_to_definition_identity(definition, other);
    if other.boss_cell_no > 0 {
        definition.boss_cell_no = other.boss_cell_no;
    }
    if definition.required_defeat_count.is_none() {
        definition.required_defeat_count = other.required_defeat_count;
    }
    if definition.clear_to_variant_key.is_none() {
        definition.clear_to_variant_key = other.clear_to_variant_key;
    }
    merge_cells(&mut definition.cells, other.cells);
    for (cell_no, fleet) in other.enemy_fleets {
        if definition.cell(cell_no).is_none() {
            tracing::warn!("enemy fleet cell {} not in topology — dropped", cell_no);
            continue;
        }
        definition.enemy_fleets.entry(cell_no).or_insert(fleet);
    }
    for (cell_no, drops) in other.ship_drops {
        definition.ship_drops.entry(cell_no).or_insert(drops);
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
    let definition_labels = definition.label_to_cell_no();
    other
        .label_to_cell_no()
        .into_iter()
        .filter_map(|(label, other_cell_no)| {
            definition_labels
                .get(&label)
                .map(|definition_cell_no| (other_cell_no, *definition_cell_no))
        })
        .collect()
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
                if other.color_no > 0 {
                    cell.color_no = other.color_no;
                }
                if other.event_id != 0 {
                    cell.event_id = other.event_id;
                }
                if other.event_kind != 0 {
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
    use crate::codex::map::ShipDropDefinition;

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
            ..Default::default()
        }
    }

    #[test]
    fn merge_variant_definition_remaps_secondary_cells_by_node_label() {
        let mut definition = MapVariantDefinition {
            boss_cell_no: 7,
            cells: vec![
                cell(0, "Start", vec![], 0, 0, 0),
                cell(1, "A", vec![], 2, 0, 2),
                cell(2, "B", vec![], 3, 0, 3),
                cell(3, "C", vec![], 4, 1, 4),
            ],
            ..Default::default()
        };
        let other = MapVariantDefinition {
            boss_cell_no: 1,
            cells: vec![
                cell(0, "Start", vec![2, 1], 0, 0, 0),
                cell(1, "C", vec![], 5, 1, 5),
                cell(2, "A", vec![3], 4, 1, 4),
                cell(3, "B", vec![1], 4, 1, 4),
            ],
            enemy_fleets: BTreeMap::from([(
                1,
                EnemyFleetDefinition {
                    cell_no: 1,
                    battle_kind: 1,
                    formations: vec![1],
                    ..Default::default()
                },
            )]),
            ship_drops: BTreeMap::from([(1, vec![ShipDropDefinition::default()])]),
            ..Default::default()
        };

        merge_variant_definition(&mut definition, other);

        assert_eq!(definition.cell(0).unwrap().next_cells, vec![1, 3]);
        assert_eq!(definition.cell(1).unwrap().next_cells, vec![2]);
        assert_eq!(definition.cell(2).unwrap().next_cells, vec![3]);
        assert_eq!(definition.cell(3).unwrap().event_id, 5);
        // boss_cell_no: last-non-zero-wins — primary had 7, secondary had 1 (remapped to 3 via label) → 3 overwrites 7
        assert_eq!(definition.boss_cell_no, 3);
        // color_no: last-non-zero-wins — primary had 2/3/4, secondary had 4/4/5 → secondary overwrites
        assert_eq!(definition.cell(1).unwrap().color_no, 4);
        assert_eq!(definition.cell(2).unwrap().color_no, 4);
        assert_eq!(definition.cell(3).unwrap().color_no, 5);
        // event_id: non-zero overwrite — secondary (4/4/5) overwrites primary (2/3/4)
        assert_eq!(definition.cell(1).unwrap().event_id, 4);
        assert_eq!(definition.cell(2).unwrap().event_id, 4);
        assert_eq!(definition.cell(3).unwrap().event_id, 5);
        // event_kind: non-zero overwrite — secondary's 1 overwrites primary's 0
        assert_eq!(definition.cell(1).unwrap().event_kind, 1);
        assert_eq!(definition.cell(2).unwrap().event_kind, 1);
        assert_eq!(definition.cell(3).unwrap().event_kind, 1);
        assert!(definition.enemy_fleets.contains_key(&3));
        assert!(definition.ship_drops.contains_key(&3));
    }

    #[test]
    fn merge_cells_preserves_primary_event_metadata_when_secondary_is_zero() {
        let mut cells = vec![cell(1, "A", vec![], 4, 1, 2)];
        merge_cells(&mut cells, vec![cell(1, "A", vec![], 0, 0, 0)]);
        assert_eq!(cells[0].event_id, 4);
        assert_eq!(cells[0].event_kind, 1);
        assert_eq!(cells[0].color_no, 2, "color_no unchanged when secondary is 0");
    }

    #[test]
    fn merge_cells_overwrites_primary_event_metadata_when_secondary_nonzero() {
        let mut cells = vec![cell(1, "A", vec![], 4, 1, 2)];
        merge_cells(&mut cells, vec![cell(1, "A", vec![], 5, 2, 3)]);
        assert_eq!(cells[0].event_id, 5);
        assert_eq!(cells[0].event_kind, 2);
        assert_eq!(cells[0].color_no, 3);
    }

    #[test]
    fn merge_cells_secondary_wins_when_primary_is_zero() {
        let mut cells = vec![cell(1, "A", vec![], 0, 0, 0)];
        merge_cells(&mut cells, vec![cell(1, "A", vec![], 5, 2, 3)]);
        assert_eq!(cells[0].event_id, 5);
        assert_eq!(cells[0].event_kind, 2);
    }

    fn make_variant_with_cells(cells: Vec<MapCellDefinition>) -> MapVariantDefinition {
        MapVariantDefinition {
            cells,
            ..Default::default()
        }
    }

    // ── U3 tests: label_to_cell_no ───────────────────────────────────────────

    #[test]
    fn label_to_cell_no_returns_correct_mapping_for_labeled_cells() {
        let variant = make_variant_with_cells(vec![
            cell(0, "Start", vec![1], 0, 0, 0),
            cell(1, "A", vec![2], 2, 0, 2),
            cell(2, "B", vec![], 3, 0, 3),
        ]);
        let map = variant.label_to_cell_no();
        assert_eq!(map.len(), 3);
        assert_eq!(map["Start"], 0);
        assert_eq!(map["A"], 1);
        assert_eq!(map["B"], 2);
    }

    #[test]
    fn label_to_cell_no_excludes_duplicate_labels() {
        let variant = make_variant_with_cells(vec![
            cell(1, "A", vec![], 2, 0, 2),
            cell(2, "A", vec![], 3, 0, 3),
            cell(3, "B", vec![], 4, 0, 4),
        ]);
        let map = variant.label_to_cell_no();
        assert_eq!(map.len(), 1, "duplicate label A should be excluded");
        assert_eq!(map["B"], 3);
        assert!(!map.contains_key("A"));
    }

    #[test]
    fn label_to_cell_no_returns_empty_for_unlabeled_cells() {
        let variant = make_variant_with_cells(vec![
            cell(1, "", vec![], 2, 0, 2),
            cell(2, "", vec![], 3, 0, 3),
        ]);
        let map = variant.label_to_cell_no();
        assert!(map.is_empty());
    }

    #[test]
    fn label_to_cell_no_maps_start_label_to_cell_no() {
        let variant = make_variant_with_cells(vec![
            cell(5, "Start", vec![10], 0, 0, 0),
            cell(10, "Boss", vec![], 5, 1, 0),
        ]);
        let map = variant.label_to_cell_no();
        assert_eq!(map["Start"], 5);
        assert_eq!(map["Boss"], 10);
    }
}
