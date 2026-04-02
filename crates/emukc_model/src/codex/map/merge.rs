use std::collections::BTreeMap;

use super::{MapCellDefinition, MapDefinition, MapVariantDefinition};

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
	if definition.parse_warnings.is_empty() {
		definition.parse_warnings = other.parse_warnings;
	}
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
