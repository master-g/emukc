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

fn infer_event_from_color(color_no: i64) -> (i64, i64) {
    match color_no {
        0 => (0, 0),
        2 => (2, 0),
        3 => (3, 0),
        4 => (4, 1),
        5 => (5, 1),
        n if n >= 6 => (n, 1),
        _ => (0, 0),
    }
}

fn merge_capture_into_overlay(
    overlay: &mut MapCatalog,
    definition: &MapDefinition,
    stage_id: &str,
    capture: &CapturedMapStart,
) -> Result<(), String> {
    let overlay_definition = overlay.maps.entry(capture.map_id).or_insert_with(|| {
        let mut def = MapDefinition::minimal(definition.map_id);
        def.maparea_id = definition.maparea_id;
        def.mapinfo_no = definition.mapinfo_no;
        def
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

    if capture.boss_cell_no > 0 {
        stage.boss_cell_no = capture.boss_cell_no;
    }

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
                    let (event_id, event_kind) = infer_event_from_color(captured_cell.color_no);
                    existing.event_id = event_id;
                    existing.event_kind = event_kind;
                }
            }
            None => {
                let (event_id, event_kind) = infer_event_from_color(captured_cell.color_no);
                cells.insert(
                    captured_cell.cell_no,
                    emukc_model::codex::map::MapCellDefinition {
                        cell_no: captured_cell.cell_no,
                        color_no: captured_cell.color_no,
                        event_id,
                        event_kind,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_overlay::capture::CapturedMapCell;

    fn definition_1_1() -> MapDefinition {
        MapDefinition {
            map_id: 11,
            maparea_id: 1,
            mapinfo_no: 1,
            name: "1-1".to_string(),
            level: 1,
            sally_flag: vec![],
            is_event: false,
            reset_policy: Default::default(),
            airbase_count: None,
            gauge_type: None,
            gauge_count: None,
            required_defeat_count: None,
            max_hp: None,
            default_variant: String::new(),
            rank_stage_ids: BTreeMap::new(),
            variants: BTreeMap::from([(
                String::new(),
                MapVariantDefinition {
                    variant_key: String::new(),
                    boss_cell_no: 3,
                    cells: vec![
                        emukc_model::codex::map::MapCellDefinition {
                            cell_no: 0,
                            color_no: 0,
                            event_id: 0,
                            event_kind: 0,
                            next_cells: vec![1],
                            node_label: Some("Start".to_string()),
                            master_cell_id: None,
                            distance: None,
                        },
                        emukc_model::codex::map::MapCellDefinition {
                            cell_no: 1,
                            color_no: 4,
                            event_id: 4,
                            event_kind: 1,
                            next_cells: vec![2, 3],
                            node_label: None,
                            master_cell_id: None,
                            distance: None,
                        },
                        emukc_model::codex::map::MapCellDefinition {
                            cell_no: 2,
                            color_no: 4,
                            event_id: 4,
                            event_kind: 1,
                            next_cells: vec![],
                            node_label: None,
                            master_cell_id: None,
                            distance: None,
                        },
                        emukc_model::codex::map::MapCellDefinition {
                            cell_no: 3,
                            color_no: 5,
                            event_id: 5,
                            event_kind: 1,
                            next_cells: vec![],
                            node_label: None,
                            master_cell_id: None,
                            distance: None,
                        },
                    ],
                    routing_rules: BTreeMap::new(),
                    enemy_fleets: BTreeMap::new(),
                    ship_drops: BTreeMap::new(),
                    required_defeat_count: None,
                    clear_to_variant_key: None,
                    parse_warnings: Vec::new(),
                },
            )]),
        }
    }

    // --- infer_event_from_color ---

    #[test]
    fn infer_event_from_color_zero_gives_zero_zero() {
        assert_eq!(infer_event_from_color(0), (0, 0));
    }

    #[test]
    fn infer_event_from_color_two_gives_battle_kind_zero() {
        assert_eq!(infer_event_from_color(2), (2, 0));
    }

    #[test]
    fn infer_event_from_color_three_gives_kind_zero() {
        assert_eq!(infer_event_from_color(3), (3, 0));
    }

    #[test]
    fn infer_event_from_color_four_gives_kind_one() {
        assert_eq!(infer_event_from_color(4), (4, 1));
    }

    #[test]
    fn infer_event_from_color_five_gives_kind_one() {
        assert_eq!(infer_event_from_color(5), (5, 1));
    }

    #[test]
    fn infer_event_from_color_six_and_above_gives_kind_one() {
        assert_eq!(infer_event_from_color(6), (6, 1));
        assert_eq!(infer_event_from_color(10), (10, 1));
        assert_eq!(infer_event_from_color(99), (99, 1));
    }

    #[test]
    fn infer_event_from_color_negative_and_one_gives_zero_zero() {
        assert_eq!(infer_event_from_color(-1), (0, 0));
        assert_eq!(infer_event_from_color(1), (0, 0));
    }

    // --- merge_capture_into_overlay ---

    #[test]
    fn merge_capture_creates_new_stage_in_overlay() {
        let definition = definition_1_1();
        let mut overlay = MapCatalog::default();
        let capture = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![
                CapturedMapCell {
                    cell_no: 0,
                    master_cell_id: 100,
                    color_no: 2,
                    distance: None,
                },
                CapturedMapCell {
                    cell_no: 1,
                    master_cell_id: 101,
                    color_no: 4,
                    distance: Some(1),
                },
                CapturedMapCell {
                    cell_no: 3,
                    master_cell_id: 102,
                    color_no: 5,
                    distance: Some(2),
                },
            ],
        };

        let result = merge_capture_into_overlay(&mut overlay, &definition, "", &capture);
        assert!(result.is_ok());

        let overlay_def = overlay.maps.get(&11).expect("map should exist in overlay");
        let stage = overlay_def.variants.get("").expect("default stage should exist");
        assert_eq!(stage.boss_cell_no, 3);
        assert_eq!(stage.cells.len(), 3);
        assert_eq!(stage.cell(0).unwrap().master_cell_id, Some(100));
        assert_eq!(stage.cell(1).unwrap().master_cell_id, Some(101));
        assert_eq!(stage.cell(1).unwrap().distance, Some(1));
    }

    #[test]
    fn merge_capture_merges_cells_into_existing_stage() {
        let definition = definition_1_1();
        let mut overlay = MapCatalog::default();

        // First capture: only cells 0 and 1
        let capture1 = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![
                CapturedMapCell {
                    cell_no: 0,
                    master_cell_id: 100,
                    color_no: 2,
                    distance: None,
                },
                CapturedMapCell {
                    cell_no: 1,
                    master_cell_id: 101,
                    color_no: 4,
                    distance: Some(1),
                },
            ],
        };
        merge_capture_into_overlay(&mut overlay, &definition, "", &capture1).unwrap();

        // Second capture: cell 3 new + cell 1 already known (should merge distance)
        let capture2 = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![
                CapturedMapCell {
                    cell_no: 1,
                    master_cell_id: 101,
                    color_no: 4,
                    distance: Some(1),
                },
                CapturedMapCell {
                    cell_no: 3,
                    master_cell_id: 102,
                    color_no: 5,
                    distance: Some(2),
                },
            ],
        };
        merge_capture_into_overlay(&mut overlay, &definition, "", &capture2).unwrap();

        let stage = overlay.maps[&11].variants.get("").unwrap();
        assert_eq!(stage.cells.len(), 3);
        assert_eq!(stage.cell(1).unwrap().distance, Some(1));
        assert_eq!(stage.cell(3).unwrap().master_cell_id, Some(102));
    }

    #[test]
    fn merge_capture_rejects_conflicting_master_cell_id() {
        let definition = definition_1_1();
        let mut overlay = MapCatalog::default();

        let capture1 = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 100,
                color_no: 2,
                distance: None,
            }],
        };
        merge_capture_into_overlay(&mut overlay, &definition, "", &capture1).unwrap();

        let capture2 = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 999,
                color_no: 2,
                distance: None,
            }],
        };
        let result = merge_capture_into_overlay(&mut overlay, &definition, "", &capture2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("conflicting_master_cell_id"));
    }

    #[test]
    fn merge_capture_rejects_conflicting_distance() {
        let definition = definition_1_1();
        let mut overlay = MapCatalog::default();

        let capture1 = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 100,
                color_no: 2,
                distance: Some(5),
            }],
        };
        merge_capture_into_overlay(&mut overlay, &definition, "", &capture1).unwrap();

        let capture2 = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 100,
                color_no: 2,
                distance: Some(3),
            }],
        };
        let result = merge_capture_into_overlay(&mut overlay, &definition, "", &capture2);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("conflicting_distance"));
    }

    #[test]
    fn merge_capture_preserves_existing_distance_when_capture_missing() {
        let definition = definition_1_1();
        let mut overlay = MapCatalog::default();

        let capture1 = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 100,
                color_no: 2,
                distance: Some(5),
            }],
        };
        merge_capture_into_overlay(&mut overlay, &definition, "", &capture1).unwrap();

        // Second capture has no distance for cell 0
        let capture2 = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 100,
                color_no: 2,
                distance: None,
            }],
        };
        merge_capture_into_overlay(&mut overlay, &definition, "", &capture2).unwrap();

        let cell = overlay.maps[&11].variants[""].cell(0).unwrap();
        assert_eq!(cell.distance, Some(5));
    }

    #[test]
    fn merge_capture_fills_color_from_capture_when_existing_is_zero() {
        let definition = definition_1_1();
        let mut overlay = MapCatalog::default();

        let capture = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 100,
                color_no: 4,
                distance: None,
            }],
        };
        merge_capture_into_overlay(&mut overlay, &definition, "", &capture).unwrap();

        let cell = overlay.maps[&11].variants[""].cell(0).unwrap();
        assert_eq!(cell.color_no, 4);
        assert_eq!(cell.event_id, 4);
        assert_eq!(cell.event_kind, 1);
    }

    // --- build_public_map_catalog_overlay_from_captures ---

    #[test]
    fn build_overlay_rejects_err_captures() {
        let catalog = MapCatalog::default();
        let captures = vec![("bad_source".to_string(), Err("parse_error".to_string()))];

        let output = build_public_map_catalog_overlay_from_captures(&catalog, 1, captures);

        assert_eq!(output.report.accepted_records.len(), 0);
        assert_eq!(output.report.rejected_records.len(), 1);
        assert_eq!(output.report.rejected_records[0].reason, "parse_error");
    }

    #[test]
    fn build_overlay_rejects_unknown_map_id() {
        let catalog = MapCatalog::default();
        let capture = CapturedMapStart {
            map_id: 9999,
            boss_cell_no: 1,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 1,
                color_no: 2,
                distance: None,
            }],
        };
        let captures = vec![("source".to_string(), Ok(capture))];

        let output = build_public_map_catalog_overlay_from_captures(&catalog, 1, captures);

        assert_eq!(output.report.accepted_records.len(), 0);
        assert_eq!(output.report.rejected_records.len(), 1);
        assert_eq!(output.report.rejected_records[0].reason, "map_not_found");
    }

    #[test]
    fn build_overlay_reports_uncovered_stages() {
        let mut catalog = MapCatalog::default();
        catalog.maps.insert(11, definition_1_1());

        let capture = CapturedMapStart {
            map_id: 11,
            boss_cell_no: 3,
            request_path: None,
            cells: vec![CapturedMapCell {
                cell_no: 0,
                master_cell_id: 100,
                color_no: 2,
                distance: None,
            }],
        };
        let captures = vec![("source".to_string(), Ok(capture))];

        let output = build_public_map_catalog_overlay_from_captures(&catalog, 1, captures);

        assert_eq!(output.report.known_map_count, 1);
        assert_eq!(output.report.known_stage_count, 1);
        assert_eq!(output.report.covered_stage_count, 1);
        assert_eq!(output.report.covered_map_count, 1);
        assert!(output.report.uncovered_stages.is_empty());
    }
}
