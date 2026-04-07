use std::collections::{BTreeMap, BTreeSet};

use emukc_model::codex::map::{MapCatalog, MapDefinition, MapVariantDefinition};

use super::{
	MapOverlayAcceptedRecord, MapOverlayBuildOutput, MapOverlayBuildReport,
	MapOverlayRejectedRecord, capture::CapturedMapStart, matching::choose_stage_match,
};

pub(super) fn build_public_map_catalog_overlay_from_captures(
	catalog: &MapCatalog,
	discovered_sources: usize,
	captures: Vec<(String, Result<CapturedMapStart, String>)>,
) -> MapOverlayBuildOutput {
	let mut overlay = MapCatalog::default();
	let mut accepted_records = Vec::new();
	let mut rejected_records = Vec::new();
	let mut covered_stages = BTreeSet::<String>::new();

	for (source, capture) in captures {
		let capture = match capture {
			Ok(capture) => capture,
			Err(reason) => {
				rejected_records.push(MapOverlayRejectedRecord {
					source,
					reason,
					request_path: Some("/kcsapi/api_req_map/start".to_string()),
					map_id: None,
				});
				continue;
			}
		};

		let request_path = capture.request_path.clone();
		let Some(definition) = catalog.map_definition(capture.map_id) else {
			rejected_records.push(MapOverlayRejectedRecord {
				source,
				reason: "map_not_found".to_string(),
				request_path,
				map_id: Some(capture.map_id),
			});
			continue;
		};

		let stage_id = match choose_stage_match(definition, &capture) {
			Ok(stage_id) => stage_id,
			Err(reason) => {
				rejected_records.push(MapOverlayRejectedRecord {
					source,
					reason,
					request_path,
					map_id: Some(capture.map_id),
				});
				continue;
			}
		};

		if let Err(reason) =
			merge_capture_into_overlay(&mut overlay, definition, &stage_id, &capture)
		{
			rejected_records.push(MapOverlayRejectedRecord {
				source,
				reason,
				request_path,
				map_id: Some(capture.map_id),
			});
			continue;
		}

		covered_stages.insert(format!("{}:{stage_id}", capture.map_id));
		accepted_records.push(MapOverlayAcceptedRecord {
			source,
			map_id: capture.map_id,
			stage_id,
			cell_count: capture.cells.len(),
		});
	}

	let known_stage_count =
		catalog.maps.values().map(|definition| definition.variants.len()).sum::<usize>();
	let uncovered_stages = catalog
		.maps
		.values()
		.flat_map(|definition| {
			definition.variants.keys().map(|stage_id| format!("{}:{stage_id}", definition.map_id))
		})
		.filter(|stage_key| !covered_stages.contains(stage_key))
		.collect::<Vec<_>>();
	let covered_map_count = covered_stages
		.iter()
		.filter_map(|stage_key| stage_key.split_once(':').map(|(map_id, _)| map_id.to_string()))
		.collect::<BTreeSet<_>>()
		.len();

	MapOverlayBuildOutput {
		overlay,
		report: MapOverlayBuildReport {
			discovered_sources,
			accepted_records,
			rejected_records,
			known_map_count: catalog.maps.len(),
			known_stage_count,
			covered_map_count,
			covered_stage_count: covered_stages.len(),
			uncovered_stages,
		},
	}
}

fn merge_capture_into_overlay(
	overlay: &mut MapCatalog,
	definition: &MapDefinition,
	stage_id: &str,
	capture: &CapturedMapStart,
) -> Result<(), String> {
	let overlay_definition = overlay.maps.entry(capture.map_id).or_insert_with(|| MapDefinition {
		map_id: definition.map_id,
		maparea_id: definition.maparea_id,
		mapinfo_no: definition.mapinfo_no,
		name: String::new(),
		level: 0,
		sally_flag: Vec::new(),
		is_event: false,
		reset_policy: Default::default(),
		airbase_count: None,
		gauge_type: None,
		gauge_count: None,
		required_defeat_count: None,
		max_hp: None,
		default_variant: String::new(),
		rank_stage_ids: BTreeMap::new(),
		variants: BTreeMap::new(),
	});
	let stage = overlay_definition.variants.entry(stage_id.to_string()).or_insert_with(|| {
		MapVariantDefinition {
			variant_key: stage_id.to_string(),
			boss_cell_no: 0,
			cells: Vec::new(),
			routing_rules: BTreeMap::new(),
			enemy_fleets: BTreeMap::new(),
			ship_drops: BTreeMap::new(),
			required_defeat_count: None,
			clear_to_variant_key: None,
			parse_warnings: Vec::new(),
		}
	});

	let mut cells =
		stage.cells.iter().cloned().map(|cell| (cell.cell_no, cell)).collect::<BTreeMap<_, _>>();

	for captured_cell in &capture.cells {
		match cells.get_mut(&captured_cell.cell_no) {
			Some(existing) => {
				if existing
					.master_cell_id
					.is_some_and(|value| value != captured_cell.master_cell_id)
				{
					return Err(format!("conflicting_master_cell_id:{}", captured_cell.cell_no));
				}
				if existing.distance.is_some()
					&& captured_cell.distance.is_some()
					&& existing.distance != captured_cell.distance
				{
					return Err(format!("conflicting_distance:{}", captured_cell.cell_no));
				}
				existing.master_cell_id.get_or_insert(captured_cell.master_cell_id);
				if existing.distance.is_none() {
					existing.distance = captured_cell.distance;
				}
				if existing.color_no <= 0 && captured_cell.color_no > 0 {
					existing.color_no = captured_cell.color_no;
				}
			}
			None => {
				cells.insert(
					captured_cell.cell_no,
					emukc_model::codex::map::MapCellDefinition {
						cell_no: captured_cell.cell_no,
						color_no: captured_cell.color_no,
						event_id: 0,
						event_kind: 0,
						next_cells: Vec::new(),
						node_label: None,
						master_cell_id: Some(captured_cell.master_cell_id),
						distance: captured_cell.distance,
					},
				);
			}
		}
	}

	stage.cells = cells.into_values().collect();
	Ok(())
}
