//! Source robustness cross-check (U6, R6): cross-reference the parsed wikiwiki map
//! catalog against `real_map_start_data` over the surface the two sources **actually
//! share**, so a source-level divergence is flagged instead of silently trusted.
//!
//! This mirrors [`crate::battle_rules`] / [`crate::map_route_rules`]'s
//! validator / finding / report / severity shape, but it is a **consistency linter over a
//! thin shared surface**, not a full structural cross-check.
//!
//! ## Bounded scope (be scope-honest)
//!
//! The wikiwiki catalog ([`MapCatalog`] of [`MapDefinition`] / [`MapVariantDefinition`])
//! carries full routing rules, per-cell enemy fleets, and cells. The
//! `real_map_start_data` captures (`api_req_map/start` responses) carry far less — only
//! `api_id` / `api_no` / `api_color_no` / `api_passed` per cell plus `api_bosscell_no` and
//! sparse `api_e_deck_info` start-cell encounters. They have **no routing edges and no
//! per-cell enemy fleets**. So the cross-check is bounded to what BOTH sources share:
//!
//! - **cell-number sets** per map (cells present in one source but not the other → divergence)
//! - **boss-cell identity** per map (the real-start `api_bosscell_no` vs the wikiwiki
//!   variant's `boss_cell_no`)
//!
//! A map present in one source but not the other is reported as an **informational
//! source-gap**, not a hard failure.
//!
//! Deeper comparison (routing edges, per-cell enemy fleets, semantic encounter equivalence)
//! requires a second source that carries that data (e.g. decoded `main.js` map structures)
//! and is explicitly **deferred**. Start-cell encounter ship-id overlap is also deferred:
//! the real-start `api_e_deck_info` carries it, but the wikiwiki catalog keys enemy fleets
//! per battle cell without a comparable "start-cell encounter" surface, so there is no
//! shared surface to compare against in v1.
//!
//! Note on boss identity: `api_color_no == 5` marks boss-*class* cells in the real data, but
//! a map can carry several `color_no == 5` cells (e.g. boss + secondary boss-class nodes), so
//! it does **not** uniquely identify the boss. The canonical boss cell is `api_bosscell_no`,
//! which is what this cross-check compares.
#![allow(missing_docs)]

use std::collections::BTreeSet;

use emukc_model::codex::map::MapCatalog;
use serde::{Deserialize, Serialize};

use crate::map_overlay::capture::{CapturedMapStart, load_embedded_real_map_start_capture};
use crate::real_map_start_asset::RealMapStartAsset;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceCrosscheckSeverity {
    /// A genuine source-level divergence over the shared surface.
    Error,
    /// A map present in only one source (a source-gap). Informational, not a failure.
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceCrosscheckFindingKind {
    /// A cell number present in one source's cell set but absent from the other's.
    CellSetDivergence,
    /// The two sources disagree on the boss cell number.
    BossCellDivergence,
    /// A map is present in the wikiwiki catalog but has no `real_map_start_data` capture.
    MapMissingFromRealStart,
    /// A map has a `real_map_start_data` capture but is absent from the wikiwiki catalog.
    MapMissingFromWikiwiki,
    /// A `real_map_start_data` capture could not be parsed (e.g. an error capture); it is
    /// skipped from comparison rather than silently trusted.
    RealStartCaptureUnusable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCrosscheckFinding {
    pub severity: SourceCrosscheckSeverity,
    pub kind: SourceCrosscheckFindingKind,
    pub map_id: i64,
    /// The cell number(s) involved, where applicable (cell-set / boss-cell findings).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cell_nos: Vec<i64>,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCrosscheckReport {
    pub findings: Vec<SourceCrosscheckFinding>,
}

impl SourceCrosscheckReport {
    /// True if any finding is an actual divergence (`Error`); source-gaps (`Info`) do not count.
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|f| f.severity == SourceCrosscheckSeverity::Error)
    }

    /// Findings that are genuine divergences (severity `Error`).
    pub fn divergences(&self) -> impl Iterator<Item = &SourceCrosscheckFinding> {
        self.findings.iter().filter(|f| f.severity == SourceCrosscheckSeverity::Error)
    }

    fn push_error(
        &mut self,
        kind: SourceCrosscheckFindingKind,
        map_id: i64,
        cell_nos: Vec<i64>,
        message: impl Into<String>,
    ) {
        self.findings.push(SourceCrosscheckFinding {
            severity: SourceCrosscheckSeverity::Error,
            kind,
            map_id,
            cell_nos,
            message: message.into(),
        });
    }

    fn push_info(
        &mut self,
        kind: SourceCrosscheckFindingKind,
        map_id: i64,
        message: impl Into<String>,
    ) {
        self.findings.push(SourceCrosscheckFinding {
            severity: SourceCrosscheckSeverity::Info,
            kind,
            map_id,
            cell_nos: Vec::new(),
            message: message.into(),
        });
    }
}

/// The wikiwiki side of the shared surface for one map: its cell-number set and boss cell.
///
/// Taken from the map's default variant; the real-start capture is also a single
/// (default) routing snapshot, so comparing the default variant is the apples-to-apples
/// surface. Non-default (event/P-unlock) variants are out of scope for v1.
fn wikiwiki_shared_surface(catalog: &MapCatalog, map_id: i64) -> Option<(BTreeSet<i64>, i64)> {
    let definition = catalog.maps.get(&map_id)?;
    let variant = definition.active_stage(None)?;
    let cells: BTreeSet<i64> = variant.cells.iter().map(|c| c.cell_no).collect();
    Some((cells, variant.boss_cell_no))
}

/// Cross-check the parsed wikiwiki [`MapCatalog`] against the embedded `real_map_start_data`
/// assets over the shared surface (cell-number sets + boss-cell identity).
///
/// See the module docs for the bounded scope. Maps present in only one source are reported as
/// `Info` source-gaps, not errors.
pub fn crosscheck_map_sources_embedded(catalog: &MapCatalog) -> SourceCrosscheckReport {
    crosscheck_map_sources(catalog, crate::real_map_start_asset::EMBEDDED_REAL_MAP_START_ASSETS)
}

/// Cross-check the parsed wikiwiki [`MapCatalog`] against a given set of real-start assets.
pub fn crosscheck_map_sources(
    catalog: &MapCatalog,
    real_start_assets: &[RealMapStartAsset],
) -> SourceCrosscheckReport {
    let mut report = SourceCrosscheckReport::default();
    let mut real_map_ids: BTreeSet<i64> = BTreeSet::new();

    for asset in real_start_assets {
        // Parse reuses the map_overlay capture parser (single source of truth for the
        // real_map_start schema). An unparseable / error capture is surfaced, not skipped.
        let capture = match load_embedded_real_map_start_capture(asset) {
            Ok((_, Ok(capture))) => capture,
            Ok((_, Err(reason))) => {
                report.push_info(
                    SourceCrosscheckFindingKind::RealStartCaptureUnusable,
                    0,
                    format!("real_map_start asset {} is unusable: {reason}", asset.name),
                );
                continue;
            }
            Err(err) => {
                report.push_info(
                    SourceCrosscheckFindingKind::RealStartCaptureUnusable,
                    0,
                    format!("real_map_start asset {} failed to parse: {err}", asset.name),
                );
                continue;
            }
        };

        // Multiple assets can target the same map_id (e.g. map_7-3 + map_7-3-part2). Only
        // cross-check the first capture per map to keep the comparison apples-to-apples; a
        // duplicate is informational.
        if !real_map_ids.insert(capture.map_id) {
            report.push_info(
                SourceCrosscheckFindingKind::RealStartCaptureUnusable,
                capture.map_id,
                format!(
                    "real_map_start asset {} duplicates map {}; skipped from comparison",
                    asset.name, capture.map_id
                ),
            );
            continue;
        }

        crosscheck_one_map(catalog, &capture, &mut report);
    }

    // wikiwiki maps that have no real_map_start capture → source-gap (Info).
    for &map_id in catalog.maps.keys() {
        if !real_map_ids.contains(&map_id) {
            report.push_info(
                SourceCrosscheckFindingKind::MapMissingFromRealStart,
                map_id,
                format!(
                    "map {map_id} is in the wikiwiki catalog but has no real_map_start_data capture"
                ),
            );
        }
    }

    report
}

fn crosscheck_one_map(
    catalog: &MapCatalog,
    real: &CapturedMapStart,
    report: &mut SourceCrosscheckReport,
) {
    let map_id = real.map_id;
    let Some((wiki_cells, wiki_boss)) = wikiwiki_shared_surface(catalog, map_id) else {
        // real start present, wikiwiki absent → source-gap (Info).
        report.push_info(
            SourceCrosscheckFindingKind::MapMissingFromWikiwiki,
            map_id,
            format!("map {map_id} has real_map_start_data but is absent from the wikiwiki catalog"),
        );
        return;
    };

    let real_cells: BTreeSet<i64> = real.cells.iter().map(|c| c.cell_no).collect();

    // Cell-set divergence: cells present in one source but not the other.
    let only_wiki: Vec<i64> = wiki_cells.difference(&real_cells).copied().collect();
    let only_real: Vec<i64> = real_cells.difference(&wiki_cells).copied().collect();

    if !only_wiki.is_empty() {
        report.push_error(
            SourceCrosscheckFindingKind::CellSetDivergence,
            map_id,
            only_wiki.clone(),
            format!(
                "map {map_id}: cells {only_wiki:?} are in the wikiwiki catalog but not in real_map_start_data"
            ),
        );
    }
    if !only_real.is_empty() {
        report.push_error(
            SourceCrosscheckFindingKind::CellSetDivergence,
            map_id,
            only_real.clone(),
            format!(
                "map {map_id}: cells {only_real:?} are in real_map_start_data but not in the wikiwiki catalog"
            ),
        );
    }

    // Boss-cell divergence. The real-start asset may omit api_bosscell_no (0); only compare
    // when both sides declare a boss cell.
    if real.boss_cell_no != 0 && wiki_boss != 0 && real.boss_cell_no != wiki_boss {
        report.push_error(
            SourceCrosscheckFindingKind::BossCellDivergence,
            map_id,
            vec![wiki_boss, real.boss_cell_no],
            format!(
                "map {map_id}: boss cell disagrees — wikiwiki says {wiki_boss}, real_map_start_data says {}",
                real.boss_cell_no
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use emukc_model::codex::map::{MapCellDefinition, MapDefinition, MapVariantDefinition};
    use serde_json::json;

    use super::*;

    fn cell(cell_no: i64) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            ..Default::default()
        }
    }

    /// Build a single-variant wikiwiki map with the given cell numbers and boss cell.
    fn wiki_map(map_id: i64, cell_nos: &[i64], boss_cell_no: i64) -> MapDefinition {
        let mut def = MapDefinition::minimal(map_id);
        def.variants.insert(
            String::new(),
            MapVariantDefinition {
                variant_key: String::new(),
                boss_cell_no,
                cells: cell_nos.iter().copied().map(cell).collect(),
                ..Default::default()
            },
        );
        def
    }

    fn catalog_with(maps: Vec<MapDefinition>) -> MapCatalog {
        let mut catalog = MapCatalog::default();
        for map in maps {
            catalog.maps.insert(map.map_id, map);
        }
        catalog
    }

    /// Build a synthetic `real_map_start` asset (the embedded-asset JSON envelope shape).
    fn real_start_asset(
        name: &'static str,
        maparea_id: i64,
        mapinfo_no: i64,
        boss_cell_no: i64,
        cell_nos: &[i64],
    ) -> RealMapStartAsset {
        let cell_data: Vec<_> = cell_nos
            .iter()
            .enumerate()
            .map(|(i, &no)| {
                json!({
                    "api_id": 3000 + i as i64 + 1,
                    "api_no": no,
                    "api_color_no": if no == boss_cell_no { 5 } else { 4 },
                    "api_passed": 0,
                })
            })
            .collect();
        let body = json!({
            "api_result": 1,
            "api_result_msg": "成功",
            "api_data": {
                "api_cell_data": cell_data,
                "api_maparea_id": maparea_id,
                "api_mapinfo_no": mapinfo_no,
                "api_bosscell_no": boss_cell_no,
            }
        });
        // RealMapStartAsset::new borrows &'static str; leak the synthetic JSON for the test.
        let leaked: &'static str = Box::leak(body.to_string().into_boxed_str());
        RealMapStartAsset::new(name, leaked)
    }

    // ── Happy: both sources agree on cell set + boss cell → zero divergences ──
    #[test]
    fn source_crosscheck_agreeing_sources_have_no_divergences() {
        let catalog = catalog_with(vec![wiki_map(11, &[0, 1, 2, 3], 3)]);
        let assets = [real_start_asset("map_1-1.json", 1, 1, 3, &[0, 1, 2, 3])];

        let report = crosscheck_map_sources(&catalog, &assets);

        assert!(!report.has_errors(), "expected no divergences, got {:?}", report.findings);
        assert_eq!(report.divergences().count(), 0);
    }

    // ── Error: a cell present in one source but not the other → named divergence ──
    #[test]
    fn source_crosscheck_cell_set_gap_is_a_divergence() {
        // wikiwiki declares cell 4; real start data does not.
        let catalog = catalog_with(vec![wiki_map(11, &[0, 1, 2, 3, 4], 3)]);
        let assets = [real_start_asset("map_1-1.json", 1, 1, 3, &[0, 1, 2, 3])];

        let report = crosscheck_map_sources(&catalog, &assets);

        assert!(report.has_errors());
        let finding = report
            .divergences()
            .find(|f| f.kind == SourceCrosscheckFindingKind::CellSetDivergence)
            .expect("expected a CellSetDivergence finding");
        assert_eq!(finding.map_id, 11);
        assert_eq!(finding.cell_nos, vec![4], "divergence must name the cell-number delta");
    }

    // ── Error: boss cells disagree → named divergence ──
    #[test]
    fn source_crosscheck_boss_cell_mismatch_is_a_divergence() {
        let catalog = catalog_with(vec![wiki_map(11, &[0, 1, 2, 3], 3)]);
        // Same cells, different boss cell.
        let assets = [real_start_asset("map_1-1.json", 1, 1, 2, &[0, 1, 2, 3])];

        let report = crosscheck_map_sources(&catalog, &assets);

        assert!(report.has_errors());
        let finding = report
            .divergences()
            .find(|f| f.kind == SourceCrosscheckFindingKind::BossCellDivergence)
            .expect("expected a BossCellDivergence finding");
        assert_eq!(finding.map_id, 11);
        assert!(finding.cell_nos.contains(&3) && finding.cell_nos.contains(&2));
    }

    // ── Edge: a map present in only one source → source-gap (Info), not a failure ──
    #[test]
    fn source_crosscheck_map_only_in_wikiwiki_is_a_source_gap_not_a_failure() {
        // Map 12 is in wikiwiki only; map 11 is in both and agrees.
        let catalog = catalog_with(vec![wiki_map(11, &[0, 1], 1), wiki_map(12, &[0, 1], 1)]);
        let assets = [real_start_asset("map_1-1.json", 1, 1, 1, &[0, 1])];

        let report = crosscheck_map_sources(&catalog, &assets);

        assert!(!report.has_errors(), "a source-gap must not be an error: {:?}", report.findings);
        let gap = report
            .findings
            .iter()
            .find(|f| f.kind == SourceCrosscheckFindingKind::MapMissingFromRealStart)
            .expect("expected a MapMissingFromRealStart source-gap");
        assert_eq!(gap.severity, SourceCrosscheckSeverity::Info);
        assert_eq!(gap.map_id, 12);
    }

    // ── Edge: a map present only in real start data → source-gap (Info), not a failure ──
    #[test]
    fn source_crosscheck_map_only_in_real_start_is_a_source_gap_not_a_failure() {
        let catalog = catalog_with(vec![]);
        let assets = [real_start_asset("map_1-1.json", 1, 1, 1, &[0, 1])];

        let report = crosscheck_map_sources(&catalog, &assets);

        assert!(!report.has_errors());
        let gap = report
            .findings
            .iter()
            .find(|f| f.kind == SourceCrosscheckFindingKind::MapMissingFromWikiwiki)
            .expect("expected a MapMissingFromWikiwiki source-gap");
        assert_eq!(gap.severity, SourceCrosscheckSeverity::Info);
        assert_eq!(gap.map_id, 11);
    }

    // ── Edge: an error-capture real_map_start asset is surfaced, not silently trusted ──
    #[test]
    fn source_crosscheck_unusable_real_start_capture_is_surfaced() {
        let catalog = catalog_with(vec![]);
        let asset = RealMapStartAsset::new(
            "map_7-4.json",
            r#"{"api_result":100,"api_result_msg":"please re-login"}"#,
        );

        let report = crosscheck_map_sources(&catalog, std::slice::from_ref(&asset));

        let finding = report
            .findings
            .iter()
            .find(|f| f.kind == SourceCrosscheckFindingKind::RealStartCaptureUnusable)
            .expect("expected an unusable-capture finding");
        assert_eq!(finding.severity, SourceCrosscheckSeverity::Info);
    }

    // ── Live signal: cross-check the embedded wikiwiki catalog vs real_map_start_data ──
    //
    // Both sides are compiled-in repo assets (no .data/codex needed). This reports whether
    // the live sources actually diverge on the shared surface. It is intentionally NOT an
    // assertion that they agree — a real divergence is signal about source health, surfaced
    // via the printed report rather than a red test (the synthetic tests above guard the
    // detector itself).
    #[test]
    fn source_crosscheck_live_embedded_sources_report() {
        let raw = include_str!("../assets/wikiwiki_map_catalog.json");
        let catalog: MapCatalog =
            serde_json::from_str(raw).expect("embedded wikiwiki_map_catalog.json must parse");

        let report = crosscheck_map_sources_embedded(&catalog);

        let divergences: Vec<_> = report.divergences().collect();
        eprintln!(
            "live source cross-check: {} divergence(s), {} total finding(s)",
            divergences.len(),
            report.findings.len()
        );
        for d in &divergences {
            eprintln!("  DIVERGENCE [{:?}] {}", d.kind, d.message);
        }
    }
}
