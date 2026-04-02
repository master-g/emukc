#![allow(missing_docs)]

use std::path::{Path, PathBuf};

use emukc_model::codex::map::MapCatalog;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::real_map_start_asset::RealMapStartAsset;

mod capture;
mod matching;
mod merge;

/// Canonical repo-tracked public map overlay asset path.
pub fn repo_public_map_catalog_overlay_path() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/public_map_catalog_overlays.json")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapOverlayAcceptedRecord {
	pub source: String,
	pub map_id: i64,
	pub stage_id: String,
	pub cell_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapOverlayRejectedRecord {
	pub source: String,
	pub reason: String,
	pub request_path: Option<String>,
	pub map_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapOverlayBuildReport {
	pub discovered_sources: usize,
	pub accepted_records: Vec<MapOverlayAcceptedRecord>,
	pub rejected_records: Vec<MapOverlayRejectedRecord>,
	pub known_map_count: usize,
	pub known_stage_count: usize,
	pub covered_map_count: usize,
	pub covered_stage_count: usize,
	pub uncovered_stages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapOverlayBuildOutput {
	pub overlay: MapCatalog,
	pub report: MapOverlayBuildReport,
}

#[derive(Debug, Error)]
pub enum MapOverlayBuildError {
	#[error("failed to read overlay source {path}: {source}")]
	Io {
		path: PathBuf,
		source: std::io::Error,
	},
	#[error("failed to parse overlay source {path}: {source}")]
	Json {
		path: PathBuf,
		source: serde_json::Error,
	},
}

pub fn build_public_map_catalog_overlay_from_response_saver_dir(
	catalog: &MapCatalog,
	fixtures_root: impl AsRef<Path>,
) -> Result<MapOverlayBuildOutput, MapOverlayBuildError> {
	let fixture_paths = capture::collect_json_files(fixtures_root.as_ref())?;
	let captures = fixture_paths
		.iter()
		.map(|path| capture::load_response_saver_capture(path))
		.collect::<Result<Vec<_>, _>>()?;
	Ok(merge::build_public_map_catalog_overlay_from_captures(
		catalog,
		fixture_paths.len(),
		captures,
	))
}

pub fn build_public_map_catalog_overlay_from_embedded_real_map_start_assets(
	catalog: &MapCatalog,
	assets: &[RealMapStartAsset],
) -> Result<MapOverlayBuildOutput, MapOverlayBuildError> {
	let captures = assets
		.iter()
		.map(capture::load_embedded_real_map_start_capture)
		.collect::<Result<Vec<_>, _>>()?;
	Ok(merge::build_public_map_catalog_overlay_from_captures(catalog, assets.len(), captures))
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;

	use emukc_model::codex::map::{MapDefinition, MapVariantDefinition};

	use super::*;

	fn sample_catalog() -> MapCatalog {
		let mut catalog = MapCatalog::default();
		catalog.maps.insert(
			11,
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
								master_cell_id: None,
								distance: None,
							},
							emukc_model::codex::map::MapCellDefinition {
								cell_no: 1,
								color_no: 4,
								event_id: 4,
								event_kind: 1,
								next_cells: vec![2, 3],
								master_cell_id: None,
								distance: None,
							},
							emukc_model::codex::map::MapCellDefinition {
								cell_no: 2,
								color_no: 4,
								event_id: 4,
								event_kind: 1,
								next_cells: vec![],
								master_cell_id: None,
								distance: None,
							},
							emukc_model::codex::map::MapCellDefinition {
								cell_no: 3,
								color_no: 5,
								event_id: 5,
								event_kind: 1,
								next_cells: vec![],
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
			},
		);
		catalog
	}

	#[test]
	fn parses_embedded_real_map_start_fixture() {
		let asset = RealMapStartAsset::new(
			"map_6-4.json",
			include_str!("../assets/real_map_start_data/map_6-4.json"),
		);
		let (_, capture) = capture::load_embedded_real_map_start_capture(&asset).unwrap();
		let capture = capture.unwrap();

		assert_eq!(capture.map_id, 64);
		assert_eq!(capture.cells.first().unwrap().master_cell_id, 411);
		assert_eq!(capture.cells.first().unwrap().cell_no, 0);
		assert_eq!(capture.cells.get(1).unwrap().distance, Some(1));
	}

	#[test]
	fn builds_overlay_from_response_saver_fixture_dir() {
		let root = tempfile::tempdir().unwrap();
		std::fs::create_dir_all(root.path()).unwrap();
		std::fs::write(
			root.path().join("api_req_map_start_11.json"),
			include_str!("../tests/fixtures/map_overlay/api_req_map_start_11.json"),
		)
		.unwrap();

		let output = build_public_map_catalog_overlay_from_response_saver_dir(
			&sample_catalog(),
			root.path(),
		)
		.unwrap();

		assert_eq!(output.report.discovered_sources, 1);
		assert_eq!(output.report.accepted_records.len(), 1);
		assert!(output.report.rejected_records.is_empty());
		let stage = output.overlay.map_definition(11).unwrap().variant("").unwrap();
		let cell0 = stage.cell(0).unwrap();
		let cell1 = stage.cell(1).unwrap();
		assert_eq!(cell0.master_cell_id, Some(3001));
		assert_eq!(cell1.master_cell_id, Some(3002));
		assert_eq!(cell1.distance, Some(2));
	}

	#[test]
	fn builds_overlay_from_embedded_real_assets() {
		let output = build_public_map_catalog_overlay_from_embedded_real_map_start_assets(
			&sample_catalog(),
			&[
				RealMapStartAsset::new(
					"map_1-1.json",
					include_str!("../assets/real_map_start_data/map_1-1.json"),
				),
				RealMapStartAsset::new(
					"map_7-4.json",
					include_str!("../assets/real_map_start_data/map_7-4.json"),
				),
			],
		)
		.unwrap();

		assert_eq!(output.report.discovered_sources, 2);
		assert_eq!(output.report.accepted_records.len(), 1);
		assert_eq!(output.report.rejected_records.len(), 1);
		assert_eq!(output.report.rejected_records[0].reason, "invalid_api_result:100");
	}

	#[test]
	fn single_stage_maps_can_bind_without_exact_cell_match() {
		let mut catalog = sample_catalog();
		catalog.maps.insert(
			12,
			MapDefinition {
				map_id: 12,
				maparea_id: 1,
				mapinfo_no: 2,
				name: "1-2".to_string(),
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
						boss_cell_no: 5,
						cells: (0..=5)
							.map(|cell_no| emukc_model::codex::map::MapCellDefinition {
								cell_no,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: Vec::new(),
								master_cell_id: None,
								distance: None,
							})
							.collect(),
						routing_rules: BTreeMap::new(),
						enemy_fleets: BTreeMap::new(),
						ship_drops: BTreeMap::new(),
						required_defeat_count: None,
						clear_to_variant_key: None,
						parse_warnings: Vec::new(),
					},
				)]),
			},
		);
		let asset = RealMapStartAsset::new(
			"map_1-2.json",
			include_str!("../assets/real_map_start_data/map_1-2.json"),
		);
		let (_, capture) = capture::load_embedded_real_map_start_capture(&asset).unwrap();
		let capture = capture.unwrap();

		let stage_id =
			matching::choose_stage_match(catalog.map_definition(12).unwrap(), &capture).unwrap();

		assert_eq!(stage_id, "");
	}

	#[test]
	fn multi_stage_superset_tie_prefers_default_stage() {
		let definition = MapDefinition {
			map_id: 73,
			maparea_id: 7,
			mapinfo_no: 3,
			name: "7-3".to_string(),
			level: 1,
			sally_flag: vec![],
			is_event: false,
			reset_policy: Default::default(),
			airbase_count: None,
			gauge_type: None,
			gauge_count: None,
			required_defeat_count: None,
			max_hp: None,
			default_variant: "pre_p_unlock".to_string(),
			rank_stage_ids: BTreeMap::new(),
			variants: BTreeMap::from([
				(
					"pre_p_unlock".to_string(),
					MapVariantDefinition {
						variant_key: "pre_p_unlock".to_string(),
						cells: (0..=16)
							.map(|cell_no| emukc_model::codex::map::MapCellDefinition {
								cell_no,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: Vec::new(),
								master_cell_id: None,
								distance: None,
							})
							.collect(),
						..Default::default()
					},
				),
				(
					"post_p_unlock".to_string(),
					MapVariantDefinition {
						variant_key: "post_p_unlock".to_string(),
						cells: (0..=16)
							.map(|cell_no| emukc_model::codex::map::MapCellDefinition {
								cell_no,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: Vec::new(),
								master_cell_id: None,
								distance: None,
							})
							.collect(),
						..Default::default()
					},
				),
			]),
		};
		let asset = RealMapStartAsset::new(
			"map_7-3.json",
			include_str!("../assets/real_map_start_data/map_7-3.json"),
		);
		let (_, capture) = capture::load_embedded_real_map_start_capture(&asset).unwrap();
		let capture = capture.unwrap();

		let stage_id = matching::choose_stage_match(&definition, &capture).unwrap();

		assert_eq!(stage_id, "pre_p_unlock");
	}
}
