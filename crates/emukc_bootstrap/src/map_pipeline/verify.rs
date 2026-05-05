//! Topology verification: real-game cell data vs. map catalog.

#[cfg(test)]
mod tests {
	use crate::real_map_start_asset::EMBEDDED_REAL_MAP_START_ASSETS;
	use emukc_model::codex::map::MapCatalog;

	fn load_catalog() -> MapCatalog {
		let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
			.join("../../.data/codex/map_catalog.json");
		if !path.exists() {
			eprintln!("skipping topology verify: {}", path.display());
			return MapCatalog::default();
		}
		let raw = std::fs::read_to_string(&path).unwrap();
		serde_json::from_str(&raw).unwrap()
	}

	fn parse_capture(asset: &crate::prelude::RealMapStartAsset) -> Option<(i64, Vec<(i64, i64, i64)>, i64)> {
		let envelope: serde_json::Value = serde_json::from_str(asset.raw_json()).ok()?;
		let api_data = envelope.get("api_data")?;
		let maparea = api_data.get("api_maparea_id")?.as_i64()?;
		let mapinfo = api_data.get("api_mapinfo_no")?.as_i64()?;
		let map_id = maparea * 10 + mapinfo;
		let boss = api_data.get("api_bosscell_no")?.as_i64()?;
		let cells = api_data
			.get("api_cell_data")?
			.as_array()?
			.iter()
			.filter_map(|c| {
				let no = c.get("api_no")?.as_i64()?;
				let id = c.get("api_id")?.as_i64()?;
				let color = c.get("api_color_no")?.as_i64()?;
				Some((no, id, color))
			})
			.collect::<Vec<_>>();
		Some((map_id, cells, boss))
	}

	#[test]
	fn real_game_cells_match_catalog_cell_no_and_color() {
		let catalog = load_catalog();
		if catalog.maps.is_empty() {
			return;
		}
		let mut checked = 0;
		let mut mismatches = Vec::new();

		for asset in EMBEDDED_REAL_MAP_START_ASSETS {
			let Some((map_id, real_cells, real_boss)) = parse_capture(asset) else {
				continue;
			};
			let Some(definition) = catalog.maps.get(&map_id) else {
				mismatches.push(format!("map {map_id}: not in catalog"));
				continue;
			};
			let Some(variant) = definition.variants.get("") else {
				mismatches.push(format!("map {map_id}: no default variant"));
				continue;
			};
			// Skip captures that belong to a different variant (cell count mismatch).
			if variant.cells.len() != real_cells.len() {
				continue;
			}

			// Boss cell number must match.
			if variant.boss_cell_no != real_boss {
				mismatches.push(format!(
					"map {map_id}: boss_cell_no catalog={} real={real_boss}",
					variant.boss_cell_no
				));
			}

			// Cell count must match.
			let catalog_count = variant.cells.len();
			if catalog_count != real_cells.len() {
				mismatches.push(format!(
					"map {map_id}: cell count catalog={catalog_count} real={}",
					real_cells.len()
				));
			}

			// Per-cell: cell_no, master_cell_id, color_no must match.
			for (api_no, api_id, api_color) in &real_cells {
				let Some(cell) = variant.cell(*api_no) else {
					mismatches
						.push(format!("map {map_id}: cell_no {api_no} missing from catalog"));
					continue;
				};
				let expected_mcid = cell.master_cell_id.unwrap_or(map_id * 100 + cell.cell_no);
				if expected_mcid != *api_id {
					mismatches.push(format!(
						"map {map_id}: cell {api_no} master_cell_id catalog={expected_mcid} real={api_id}"
					));
				}
				if cell.color_no != *api_color {
					mismatches.push(format!(
						"map {map_id}: cell {api_no} color_no catalog={} real={api_color}",
						cell.color_no
					));
				}
			}

			checked += 1;
		}

		if !mismatches.is_empty() {
			for m in &mismatches {
				eprintln!("  {m}");
			}
			panic!("{} mismatches across {checked} maps", mismatches.len());
		}
	}

	#[test]
	fn battle_cells_have_enemy_fleet_data() {
		let catalog = load_catalog();
		if catalog.maps.is_empty() {
			return;
		}
		let mut checked = 0;
		let mut missing = Vec::new();

		for asset in EMBEDDED_REAL_MAP_START_ASSETS {
			let Some((map_id, real_cells, _boss)) = parse_capture(asset) else {
				continue;
			};
			if map_id > 74 {
				continue;
			}
			let Some(variant) = catalog
				.maps
				.get(&map_id)
				.and_then(|m| m.variants.get(""))
			else {
				continue;
			};

			for (api_no, _, api_color) in &real_cells {
				if *api_color < 4 || *api_color > 5 {
					continue;
				}
				if !variant.enemy_fleets.contains_key(api_no) {
					let label = variant
						.cell(*api_no)
						.and_then(|c| c.node_label.clone())
						.unwrap_or_default();
					missing.push(format!(
						"map {map_id}: cell {api_no} ({label}) color={api_color} has no enemy fleet"
					));
				}
			}
			checked += 1;
		}

		if !missing.is_empty() {
			for m in &missing {
				eprintln!("  {m}");
			}
			panic!(
				"{} battle cells missing enemy fleets across {checked} maps",
				missing.len()
			);
		}
	}
}
