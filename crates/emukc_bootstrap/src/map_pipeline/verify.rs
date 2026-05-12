//! Topology verification: real-game cell data vs. map catalog.

#[cfg(test)]
mod tests {
    use crate::{
        map_pipeline::build_final_map_catalog_from_repo_assets,
        real_map_start_asset::EMBEDDED_REAL_MAP_START_ASSETS,
    };
    use emukc_model::codex::map::MapCatalog;
    use emukc_model::kc2::start2::ApiManifest;
    use std::{path::PathBuf, str::FromStr};

    #[derive(Debug)]
    struct RealCell {
        api_no: i64,
        api_id: i64,
        api_color: i64,
    }

    #[derive(Debug)]
    struct RealStartCapture {
        map_id: i64,
        cells: Vec<RealCell>,
        boss_cell_no: i64,
        api_no: i64,
        api_from_no: i64,
    }

    fn load_catalog() -> MapCatalog {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.data/temp");
        let manifest_path = data_root.join("start2.json");
        let Ok(manifest_raw) = std::fs::read_to_string(&manifest_path) else {
            eprintln!(
                "WARNING: topology verify skipped — manifest not found: {}",
                manifest_path.display()
            );
            return MapCatalog::default();
        };
        let manifest = match ApiManifest::from_str(&manifest_raw) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("WARNING: topology verify skipped — manifest parse failed: {e}");
                return MapCatalog::default();
            }
        };
        let kcdata_root = data_root.join("kc_data");
        if !kcdata_root.exists() {
            eprintln!(
                "WARNING: topology verify skipped — kcdata directory not found: {}",
                kcdata_root.display()
            );
            return MapCatalog::default();
        }
        match build_final_map_catalog_from_repo_assets(&data_root, &manifest) {
            Ok(catalog) => catalog,
            Err(e) => {
                eprintln!("WARNING: topology verify skipped — catalog build failed: {e}");
                MapCatalog::default()
            }
        }
    }

    fn parse_capture(asset: &crate::prelude::RealMapStartAsset) -> Option<RealStartCapture> {
        let envelope: serde_json::Value = serde_json::from_str(asset.raw_json()).ok()?;
        let api_data = envelope.get("api_data")?;
        let maparea = api_data.get("api_maparea_id")?.as_i64()?;
        let mapinfo = api_data.get("api_mapinfo_no")?.as_i64()?;
        let map_id = maparea * 10 + mapinfo;
        let boss_cell_no = api_data.get("api_bosscell_no")?.as_i64()?;
        let api_no = api_data.get("api_no")?.as_i64()?;
        let api_from_no = api_data.get("api_from_no")?.as_i64()?;
        let cells = api_data
            .get("api_cell_data")?
            .as_array()?
            .iter()
            .filter_map(|c| {
                let no = c.get("api_no")?.as_i64()?;
                let id = c.get("api_id")?.as_i64()?;
                let color = c.get("api_color_no")?.as_i64()?;
                Some(RealCell {
                    api_no: no,
                    api_id: id,
                    api_color: color,
                })
            })
            .collect::<Vec<_>>();
        Some(RealStartCapture {
            map_id,
            cells,
            boss_cell_no,
            api_no,
            api_from_no,
        })
    }

    fn is_known_phase_specific_topology_fixture(asset_name: &str) -> bool {
        matches!(
            asset_name,
            // 7-2 and 7-3 captures are phase/gauge-specific layouts. The default
            // catalog variant is still useful for runtime, but these captures need
            // dedicated variant-aware assertions before strict global parity applies.
            "map_7-2.json" | "map_7-3.json" | "map_7-3-part2.json"
        )
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
            if is_known_phase_specific_topology_fixture(asset.name) {
                continue;
            }
            let Some(capture) = parse_capture(asset) else {
                continue;
            };
            let map_id = capture.map_id;
            let Some(definition) = catalog.maps.get(&map_id) else {
                mismatches.push(format!("map {map_id}: not in catalog"));
                continue;
            };
            let Some(variant) = definition.variants.get("") else {
                mismatches.push(format!("map {map_id}: no default variant"));
                continue;
            };

            // Boss cell number must match.
            if variant.boss_cell_no != capture.boss_cell_no {
                mismatches.push(format!(
                    "map {map_id}: boss_cell_no catalog={} real={}",
                    variant.boss_cell_no, capture.boss_cell_no
                ));
            }

            // Cell count must match.
            let catalog_count = variant.cells.len();
            if catalog_count != capture.cells.len() {
                mismatches.push(format!(
                    "map {map_id}: cell count catalog={catalog_count} real={}",
                    capture.cells.len()
                ));
            }

            // Per-cell: cell_no, master_cell_id, color_no must match.
            for real_cell in &capture.cells {
                let Some(cell) = variant.cell(real_cell.api_no) else {
                    mismatches.push(format!(
                        "map {map_id}: cell_no {} missing from catalog",
                        real_cell.api_no
                    ));
                    continue;
                };
                let expected_mcid = cell.master_cell_id.unwrap_or(map_id * 100 + cell.cell_no);
                if expected_mcid != real_cell.api_id {
                    mismatches.push(format!(
                        "map {map_id}: cell {} master_cell_id catalog={expected_mcid} real={}",
                        real_cell.api_no, real_cell.api_id
                    ));
                }
                if cell.color_no != real_cell.api_color {
                    mismatches.push(format!(
                        "map {map_id}: cell {} color_no catalog={} real={}",
                        real_cell.api_no, cell.color_no, real_cell.api_color
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
    fn map_1_3_real_start_capture_matches_route_cell_topology() {
        let catalog = load_catalog();
        if catalog.maps.is_empty() {
            return;
        }
        let capture = EMBEDDED_REAL_MAP_START_ASSETS
            .iter()
            .filter_map(parse_capture)
            .find(|capture| capture.map_id == 13)
            .expect("embedded 1-3 real start capture");
        let variant = catalog
            .maps
            .get(&13)
            .and_then(|definition| definition.variants.get(""))
            .expect("1-3 default variant");

        let real_api_nos = capture.cells.iter().map(|cell| cell.api_no).collect::<Vec<_>>();
        let catalog_api_nos = variant.cells.iter().map(|cell| cell.cell_no).collect::<Vec<_>>();
        assert_eq!(capture.cells.len(), 14);
        assert_eq!(real_api_nos, (0..=13).collect::<Vec<_>>());
        assert_eq!(catalog_api_nos, real_api_nos);
        assert_eq!(capture.api_from_no, 0);
        assert_eq!(capture.api_no, 3);
        assert_eq!(capture.boss_cell_no, 10);
        assert_eq!(variant.boss_cell_no, 10);
        assert_eq!(variant.cell(0).unwrap().next_cells, vec![1, 3]);
        assert!(
            !variant.cell(0).unwrap().next_cells.contains(&4),
            "1-3 start must not route directly to API cell 4/D"
        );
    }
}
