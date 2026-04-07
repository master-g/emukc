//! Parsers for various data sources.

pub mod error;
pub mod kc3kai;
pub mod kcanotify;
pub mod kccp;
pub mod kcwiki;
pub mod kcwikizh_kcdata;
pub mod music;
pub mod tsunkit_quest;
pub mod wikiwiki_map;

use std::str::FromStr;

use emukc_model::{
	codex::{game_config::GameConfig, map::MapCatalog},
	kc2::navy::KcNavy,
	prelude::*,
};

#[cfg(test)]
use crate::map_pipeline::build_final_map_catalog;
use crate::map_pipeline::build_final_map_catalog_from_repo_assets;
use error::ParseError;
pub use kc3kai::parse as parse_kc3kai;
pub use kccp::quest::parse as parse_kccp_quests;
pub use kcwiki::parse as parse_kcwiki;
pub use kcwikizh_kcdata::parse as parse_kcdata;
pub use tsunkit_quest::parse as parse_tsunkit_quests;
pub use wikiwiki_map::{parse as parse_wikiwiki_map, parse_debug as parse_wikiwiki_map_debug};

fn load_map_catalog(
	dir: &std::path::Path,
	manifest: &ApiManifest,
) -> Result<MapCatalog, ParseError> {
	build_final_map_catalog_from_repo_assets(dir, manifest)
}

fn merge_manifest_ship(manifest: &mut ApiManifest, ship: ApiMstShip) {
	if let Some(existing) =
		manifest.api_mst_ship.iter_mut().find(|entry| entry.api_id == ship.api_id)
	{
		*existing = ship;
	} else {
		manifest.api_mst_ship.push(ship);
	}
}

fn merge_manifest_slotitem(manifest: &mut ApiManifest, slotitem: ApiMstSlotitem) {
	if manifest.find_slotitem(slotitem.api_id).is_none() {
		manifest.api_mst_slotitem.push(slotitem);
	}
}

/// Parse a partial codex from the given directory.
///
/// # Arguments
///
/// * `dir` - The directory to parse.
///
/// # Returns
///
/// A partial codex.
pub fn parse_partial_codex(dir: impl AsRef<std::path::Path>) -> Result<Codex, ParseError> {
	let dir = dir.as_ref();
	let mut manifest = {
		let path = dir.join("start2.json");
		let raw =
			std::fs::read_to_string(&path).map_err(|source| ParseError::io_at(&path, source))?;
		debug!("Parsing manifest from {:?}", path);
		ApiManifest::from_str(&raw).map_err(|source| ParseError::json_at(&path, source))?
	};

	let kcwiki = parse_kcwiki(dir, &manifest)?;
	for slotitem in kcwiki.enemy_manifest_slotitems.iter().cloned() {
		merge_manifest_slotitem(&mut manifest, slotitem);
	}
	for ship in kcwiki.enemy_manifest_ships.iter().cloned() {
		merge_manifest_ship(&mut manifest, ship);
	}

	let (ship_picturebook, ship_class_name) = parse_kcdata(dir.join("kc_data"), &manifest)?;
	let kccp_quests = {
		let path = dir.join("kccp_quests.json");
		let raw =
			std::fs::read_to_string(&path).map_err(|source| ParseError::io_at(&path, source))?;
		debug!("Parsing kccp quests from {:?}", path);
		parse_kccp_quests(&raw).map_err(|source| {
			ParseError::Generic(format!("failed to parse {}: {source}", path.display()))
		})?
	};
	let quest = parse_tsunkit_quests(dir.join("tsunkit_quests.json"), &manifest, &kccp_quests)?;

	let expedition_conditions = {
		let path = dir.join("kcanotify_expedition.json");
		debug!("Parsing KCanotify expedition data from {:?}", path);
		kcanotify::expedition::parse(&path)?
	};

	let music_list = music::get()?;

	let mut cache_source = CacheSource::default();
	{
		let path = dir.join("kc3kai_jp_quotes.json");
		let raw =
			std::fs::read_to_string(&path).map_err(|source| ParseError::io_at(&path, source))?;
		let cleaned = raw
			.trim_start_matches('\u{FEFF}') // UTF-8 BOM
			.trim_start_matches('\u{FFFE}') // UTF-16 BOM
			.trim_start_matches(['\0', '\x01', '\x02', '\x03', '\x04', '\x05']) // controls
			.trim_start(); // whitespace
		parse_kc3kai(cleaned, &mut cache_source).map_err(|source| {
			ParseError::Generic(format!("failed to parse {}: {source}", path.display()))
		})?;
	}

	let maps = load_map_catalog(dir, &manifest)?;

	Ok(Codex {
		manifest,
		ship_extra: kcwiki.ship_map,
		ship_class_name,
		ship_picturebook,
		slotitem_extra_info: kcwiki.slotitem_map,
		enemy_ship_extra: kcwiki.enemy_ship_map,
		quest,
		expedition_conditions,
		picturebook_extra: Kc3rdPicturebookExtra::default(),
		navy: KcNavy::default(),
		game_cfg: GameConfig::default(),
		music_list,
		maps,
		cache_source: Some(cache_source),
	})
}

#[cfg(test)]
mod tests {
	use super::*;
	use emukc_model::codex::map::{MapCellDefinition, MapDefinition};
	use std::collections::BTreeMap;

	#[test]
	fn load_map_catalog_uses_kcdata_when_repo_wikiwiki_asset_is_missing() {
		let root = tempfile::tempdir().unwrap();
		let kcdata_dir = root.path().join("kc_data/_map");
		std::fs::create_dir_all(&kcdata_dir).unwrap();

		std::fs::write(
			kcdata_dir.join("0012.yaml"),
			r#"data:
  id: 12
  name: "1-2 fallback"
  routes:
    1:
      to: 1
    2:
      from: 1
      to: 2
  cells:
    "1":
      name: "battle"
    "2":
      boss: true
"#,
		)
		.unwrap();

		let catalog = build_final_map_catalog(root.path(), &ApiManifest::default(), None).unwrap();
		let map_12 = catalog.map_definition(12).unwrap();

		assert_eq!(map_12.variant("").unwrap().boss_cell_no, 2);
	}

	#[test]
	fn load_map_catalog_uses_repo_wikiwiki_asset_and_only_adds_missing_kcdata_maps() {
		let root = tempfile::tempdir().unwrap();
		let kcdata_dir = root.path().join("kc_data/_map");
		std::fs::create_dir_all(&kcdata_dir).unwrap();

		std::fs::write(
			kcdata_dir.join("0012.yaml"),
			r#"data:
  id: 12
  name: "1-2 fallback"
  routes:
    1:
      to: 1
    2:
      from: 1
      to: 2
  cells:
    "1":
      name: "battle"
    "2":
      boss: true
"#,
		)
		.unwrap();

		let mut overlay = MapCatalog::default();
		overlay.maps.insert(
			12,
			MapDefinition {
				map_id: 12,
				maparea_id: 1,
				mapinfo_no: 2,
				name: "1-2 wikiwiki".to_string(),
				level: 1,
				sally_flag: vec![],
				is_event: false,
				reset_policy: Default::default(),
				airbase_count: None,
				gauge_type: None,
				gauge_count: Some(2),
				required_defeat_count: Some(3),
				max_hp: None,
				default_variant: "pre_p_unlock".to_string(),
				rank_stage_ids: BTreeMap::new(),
				variants: BTreeMap::from([(
					"pre_p_unlock".to_string(),
					emukc_model::codex::map::MapVariantDefinition {
						variant_key: "pre_p_unlock".to_string(),
						boss_cell_no: 9,
						cells: vec![],
						routing_rules: BTreeMap::new(),
						enemy_fleets: BTreeMap::new(),
						ship_drops: BTreeMap::new(),
						required_defeat_count: Some(3),
						clear_to_variant_key: Some("post_p_unlock".to_string()),
						parse_warnings: Vec::new(),
					},
				)]),
			},
		);
		overlay.maps.insert(
			11,
			MapDefinition {
				map_id: 11,
				maparea_id: 1,
				mapinfo_no: 1,
				name: "1-1 wikiwiki".to_string(),
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
					emukc_model::codex::map::MapVariantDefinition {
						variant_key: String::new(),
						boss_cell_no: 3,
						cells: vec![],
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

		let catalog =
			build_final_map_catalog(root.path(), &ApiManifest::default(), Some(overlay)).unwrap();
		let map_11 = catalog.map_definition(11).unwrap();
		let map_12 = catalog.map_definition(12).unwrap();

		assert_eq!(map_11.name, "1-1 wikiwiki");
		assert_eq!(map_12.default_variant, "pre_p_unlock");
		assert_eq!(map_12.gauge_count, Some(2));
		assert!(map_12.variants.contains_key("pre_p_unlock"));
		assert!(!map_12.variants.contains_key(""));
	}

	#[test]
	fn load_map_catalog_uses_kcdata_default_variant_as_structural_complement() {
		let root = tempfile::tempdir().unwrap();
		let kcdata_dir = root.path().join("kc_data/_map");
		std::fs::create_dir_all(&kcdata_dir).unwrap();

		std::fs::write(
			kcdata_dir.join("0012.yaml"),
			r#"data:
  id: 12
  name: "1-2 fallback"
  routes:
    1:
      to: 1
    2:
      from: 1
      to: 2
  cells:
    "1":
      name: "battle"
    "2":
      boss: true
"#,
		)
		.unwrap();

		let mut overlay = MapCatalog::default();
		overlay.maps.insert(
			12,
			MapDefinition {
				map_id: 12,
				maparea_id: 1,
				mapinfo_no: 2,
				name: "1-2 wikiwiki".to_string(),
				level: 1,
				sally_flag: vec![],
				is_event: false,
				reset_policy: Default::default(),
				airbase_count: None,
				gauge_type: None,
				gauge_count: Some(2),
				required_defeat_count: Some(3),
				max_hp: None,
				default_variant: "pre_p_unlock".to_string(),
				rank_stage_ids: BTreeMap::new(),
				variants: BTreeMap::from([(
					"pre_p_unlock".to_string(),
					emukc_model::codex::map::MapVariantDefinition {
						variant_key: "pre_p_unlock".to_string(),
						boss_cell_no: 0,
						cells: vec![MapCellDefinition {
							cell_no: 0,
							color_no: 0,
							event_id: 0,
							event_kind: 0,
							next_cells: vec![],
							node_label: Some("Start".to_string()),
							master_cell_id: None,
							distance: None,
						}],
						routing_rules: BTreeMap::new(),
						enemy_fleets: BTreeMap::new(),
						ship_drops: BTreeMap::new(),
						required_defeat_count: Some(3),
						clear_to_variant_key: Some("post_p_unlock".to_string()),
						parse_warnings: Vec::new(),
					},
				)]),
			},
		);

		let catalog =
			build_final_map_catalog(root.path(), &ApiManifest::default(), Some(overlay)).unwrap();
		let pre = catalog.map_definition(12).unwrap().variants.get("pre_p_unlock").unwrap();
		let cell_nos = pre.cells.iter().map(|cell| cell.cell_no).collect::<Vec<_>>();

		assert!(cell_nos.starts_with(&[0, 1, 2]));
		assert_eq!(pre.cell(0).unwrap().next_cells, vec![1]);
		assert_eq!(pre.boss_cell_no, 2);
		assert!(
			pre.cells
				.iter()
				.any(|cell| cell.cell_no == 2 && cell.event_id == 5 && cell.color_no == 5)
		);
	}

	#[test]
	fn load_map_catalog_prefers_kcdata_structural_start_over_inferred_wikiwiki_start() {
		let root = tempfile::tempdir().unwrap();
		let kcdata_dir = root.path().join("kc_data/_map");
		std::fs::create_dir_all(&kcdata_dir).unwrap();

		std::fs::write(
			kcdata_dir.join("0012.yaml"),
			r#"data:
  id: 12
  name: "1-2 fallback"
  routes:
    1:
      to: 1
    2:
      to: 3
    3:
      from: 1
      to: 2
    4:
      from: 2
      to: 3
  cells:
    "1":
      name: "battle"
    "2":
      name: "battle"
    "3":
      boss: true
"#,
		)
		.unwrap();

		let mut overlay = MapCatalog::default();
		overlay.maps.insert(
			12,
			MapDefinition {
				map_id: 12,
				maparea_id: 1,
				mapinfo_no: 2,
				name: "1-2 wikiwiki".to_string(),
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
					emukc_model::codex::map::MapVariantDefinition {
						variant_key: String::new(),
						boss_cell_no: 3,
						cells: vec![
							MapCellDefinition {
								cell_no: 0,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: vec![1, 2],
								node_label: Some("Start".to_string()),
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
								cell_no: 1,
								color_no: 4,
								event_id: 4,
								event_kind: 1,
								next_cells: vec![2],
								node_label: Some("A".to_string()),
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
								cell_no: 2,
								color_no: 4,
								event_id: 4,
								event_kind: 1,
								next_cells: vec![],
								node_label: Some("B".to_string()),
								master_cell_id: None,
								distance: None,
							},
						],
						routing_rules: BTreeMap::new(),
						enemy_fleets: BTreeMap::new(),
						ship_drops: BTreeMap::new(),
						required_defeat_count: None,
						clear_to_variant_key: None,
						parse_warnings: vec!["inferred_multi_root_start:1,2".to_string()],
					},
				)]),
			},
		);

		let catalog =
			build_final_map_catalog(root.path(), &ApiManifest::default(), Some(overlay)).unwrap();
		let stage = catalog.map_definition(12).unwrap().variant("").unwrap();

		assert_eq!(stage.cell(0).unwrap().next_cells, vec![1, 3]);
		assert!(stage.parse_warnings.iter().any(|warning| warning == "structural_start_fallback"));
		assert!(
			stage
				.parse_warnings
				.iter()
				.all(|warning| !warning.starts_with("inferred_multi_root_start"))
		);
	}

	#[test]
	fn load_map_catalog_aligns_kcdata_semantics_by_node_label() {
		let root = tempfile::tempdir().unwrap();
		let kcdata_dir = root.path().join("kc_data/_map");
		std::fs::create_dir_all(&kcdata_dir).unwrap();

		std::fs::write(
			kcdata_dir.join("0012.yaml"),
			r#"data:
  id: 12
  name: "1-2 semantic fallback"
  routes:
    1:
      to: A
    2:
      to: C
    3:
      from: A
      to: B
    4:
      from: B
      to: C
  cells:
    A:
      name: "battle"
    B:
      name: "battle"
    C:
      boss: true
"#,
		)
		.unwrap();

		let mut overlay = MapCatalog::default();
		overlay.maps.insert(
			12,
			MapDefinition {
				map_id: 12,
				maparea_id: 1,
				mapinfo_no: 2,
				name: "1-2 wikiwiki".to_string(),
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
					emukc_model::codex::map::MapVariantDefinition {
						variant_key: String::new(),
						boss_cell_no: 0,
						cells: vec![
							MapCellDefinition {
								cell_no: 0,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: vec![1, 2],
								node_label: Some("Start".to_string()),
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
								cell_no: 1,
								color_no: 4,
								event_id: 4,
								event_kind: 1,
								next_cells: vec![2],
								node_label: Some("A".to_string()),
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
								cell_no: 2,
								color_no: 4,
								event_id: 4,
								event_kind: 1,
								next_cells: vec![],
								node_label: Some("B".to_string()),
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
								cell_no: 3,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: vec![],
								node_label: Some("C".to_string()),
								master_cell_id: None,
								distance: None,
							},
						],
						routing_rules: BTreeMap::new(),
						enemy_fleets: BTreeMap::new(),
						ship_drops: BTreeMap::new(),
						required_defeat_count: None,
						clear_to_variant_key: None,
						parse_warnings: vec!["inferred_multi_root_start:1,2".to_string()],
					},
				)]),
			},
		);

		let catalog =
			build_final_map_catalog(root.path(), &ApiManifest::default(), Some(overlay)).unwrap();
		let stage = catalog.map_definition(12).unwrap().variant("").unwrap();

		assert_eq!(stage.cell(0).unwrap().next_cells, vec![1, 3]);
		assert_eq!(stage.boss_cell_no, 3);
		assert_eq!(stage.cell(3).unwrap().event_id, 5);
		assert_eq!(stage.cell(3).unwrap().color_no, 5);
		assert!(stage.parse_warnings.iter().any(|warning| warning == "structural_start_fallback"));
	}

	#[test]
	fn finalize_map_catalog_from_sources_applies_public_master_cell_ids() {
		let root = tempfile::tempdir().unwrap();
		let kcdata_dir = root.path().join("kc_data/_map");
		std::fs::create_dir_all(&kcdata_dir).unwrap();

		let mut wikiwiki_catalog = MapCatalog::default();
		wikiwiki_catalog.maps.insert(
			11,
			MapDefinition {
				map_id: 11,
				maparea_id: 1,
				mapinfo_no: 1,
				name: "1-1 wikiwiki".to_string(),
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
					emukc_model::codex::map::MapVariantDefinition {
						variant_key: String::new(),
						boss_cell_no: 3,
						cells: vec![
							MapCellDefinition {
								cell_no: 0,
								color_no: 0,
								event_id: 0,
								event_kind: 0,
								next_cells: vec![1],
								node_label: Some("Start".to_string()),
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
								cell_no: 1,
								color_no: 4,
								event_id: 4,
								event_kind: 1,
								next_cells: vec![2, 3],
								node_label: None,
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
								cell_no: 2,
								color_no: 4,
								event_id: 4,
								event_kind: 1,
								next_cells: vec![],
								node_label: None,
								master_cell_id: None,
								distance: None,
							},
							MapCellDefinition {
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
			},
		);

		let catalog =
			build_final_map_catalog(root.path(), &ApiManifest::default(), Some(wikiwiki_catalog))
				.unwrap();
		let map_11 = catalog.map_definition(11).unwrap();
		let ids = map_11
			.variant("")
			.unwrap()
			.cells
			.iter()
			.map(|cell| cell.master_cell_id)
			.collect::<Vec<_>>();

		assert_eq!(ids, vec![Some(3001), Some(3002), Some(3003), Some(3004)]);
	}
}
