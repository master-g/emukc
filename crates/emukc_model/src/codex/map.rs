#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashMap};

mod debug;
mod merge;
mod types;

#[allow(deprecated)]
use crate::profile::map_record::DEFAULT_MAP_RECORDS;
use crate::{
    kc2::start2::{ApiManifest, ApiMstMapinfo},
    profile::map_record::MapRefreshType,
};

pub use types::*;

use merge::merge_definition as merge_definition_impl;
pub use merge::{build_cell_no_map, merge_routing_overlay};

/// Warnings produced by `MapDefinition::validate()`.
#[derive(Debug, Clone, PartialEq)]
pub enum MapValidationWarning {
    SelfLoop {
        map_id: i64,
        cell_no: i64,
        variant: String,
    },
    Unreachable {
        map_id: i64,
        cell_no: i64,
        variant: String,
    },
    RuleTargetNotInNextCells {
        map_id: i64,
        from_cell_no: i64,
        to_cell_no: i64,
        variant: String,
    },
}

impl MapCatalog {
    pub fn from_manifest(manifest: &ApiManifest) -> Self {
        #[allow(deprecated)]
        let defaults = DEFAULT_MAP_RECORDS
            .iter()
            .map(|record| (record.id, record))
            .collect::<BTreeMap<_, _>>();
        let mut maps = BTreeMap::new();
        let area_types = manifest
            .api_mst_maparea
            .iter()
            .map(|area| (area.api_id, area.api_type))
            .collect::<BTreeMap<_, _>>();

        for map in &manifest.api_mst_mapinfo {
            let overlay = defaults.get(&map.api_id);
            let (reset_policy, airbase_count, gauge_type, gauge_count, required_defeat_count) =
                if let Some(record) = overlay {
                    let defeat_ctx = record.defeat_ctx.as_ref();
                    (
                        defeat_ctx
                            .map(|ctx| match ctx.refresh_type {
                                MapRefreshType::Monthly => MapResetPolicy::Monthly,
                                MapRefreshType::Never => MapResetPolicy::Never,
                            })
                            .unwrap_or(MapResetPolicy::Never),
                        record.airbase_count,
                        defeat_ctx.map(|ctx| ctx.gauge_type as i64),
                        defeat_ctx.map(|ctx| ctx.gauge_num),
                        defeat_ctx.map(|ctx| ctx.defeat_required),
                    )
                } else {
                    (MapResetPolicy::Never, None, None, None, map.api_required_defeat_count)
                };
            let is_event = area_types.get(&map.api_maparea_id).copied().unwrap_or_default() == 1;
            maps.insert(
                map.api_id,
                MapDefinition {
                    map_id: map.api_id,
                    maparea_id: map.api_maparea_id,
                    mapinfo_no: map.api_no,
                    name: map.api_name.clone(),
                    level: map.api_level,
                    sally_flag: map.api_sally_flag.clone(),
                    is_event,
                    reset_policy,
                    airbase_count,
                    gauge_type,
                    gauge_count,
                    required_defeat_count,
                    max_hp: extract_max_hp(map),
                    default_variant: String::new(),
                    rank_stage_ids: BTreeMap::new(),
                    variants: BTreeMap::new(),
                },
            );
        }

        let mut catalog = Self {
            maps,
            prerequisites: build_regular_prerequisites(),
        };
        catalog.ensure_synthetic_variants();
        catalog
    }

    pub fn map_definition(&self, map_id: i64) -> Option<&MapDefinition> {
        self.maps.get(&map_id)
    }

    /// Returns the prerequisite map ID for the given map, if any.
    /// Returns `None` for maps with no prerequisite (e.g., 1-1).
    pub fn prerequisite_for(&self, map_id: i64) -> Option<i64> {
        self.prerequisites.get(&map_id).copied()
    }

    /// Returns all map IDs whose prerequisite is the given `map_id`.
    pub fn dependents_of(&self, map_id: i64) -> Vec<i64> {
        self.prerequisites
            .iter()
            .filter(|&(_, prereq)| *prereq == map_id)
            .map(|(&dep_id, _)| dep_id)
            .collect()
    }

    pub fn map_definition_by_area_no(
        &self,
        maparea_id: i64,
        mapinfo_no: i64,
    ) -> Option<&MapDefinition> {
        self.maps.get(&compose_map_id(maparea_id, mapinfo_no))
    }

    pub fn known_maps(&self) -> Vec<&MapDefinition> {
        self.maps.values().collect()
    }

    pub fn merge_missing_from(&mut self, other: Self) {
        // Preserve prerequisites from whichever catalog has them
        if self.prerequisites.is_empty() && !other.prerequisites.is_empty() {
            self.prerequisites = other.prerequisites;
        }
        for (map_id, other_definition) in other.maps {
            match self.maps.entry(map_id) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(other_definition);
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    merge_definition_impl(entry.get_mut(), other_definition);
                }
            }
        }
        self.ensure_synthetic_variants();
    }

    pub fn to_debug_json(&self, manifest: &ApiManifest) -> serde_json::Value {
        debug::to_debug_json(self, manifest)
    }

    fn ensure_synthetic_variants(&mut self) {
        for definition in self.maps.values_mut() {
            if definition.variants.is_empty() {
                // Sentinel: boss_cell_no = 0 means "no real boss; synthetic fallback".
                // Callers must handle 0 as "unknown boss" (merge.rs already guards with > 0).
                definition.variants.insert(
                    String::new(),
                    MapVariantDefinition {
                        variant_key: String::new(),
                        boss_cell_no: 0,
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
                );
            }
        }
    }
}

impl MapDefinition {
    /// Validate graph invariants across all variants.
    /// Returns warnings for: self-loops, unreachable cells, routing-rule targets not in next_cells.
    /// Designed for warn-only use at codex load.
    pub fn validate(&self) -> Vec<MapValidationWarning> {
        let mut warnings = Vec::new();
        let map_id = self.map_id;

        for (vkey, variant) in &self.variants {
            let cell_nos: BTreeSet<i64> = variant.cells.iter().map(|c| c.cell_no).collect();
            let next_map: BTreeMap<i64, &[i64]> =
                variant.cells.iter().map(|c| (c.cell_no, c.next_cells.as_slice())).collect();

            // Self-loops
            for cell in &variant.cells {
                if cell.next_cells.contains(&cell.cell_no) {
                    warnings.push(MapValidationWarning::SelfLoop {
                        map_id,
                        cell_no: cell.cell_no,
                        variant: vkey.clone(),
                    });
                }
            }

            // Unreachable cells (no incoming edge, not cell 0)
            let mut has_incoming: BTreeSet<i64> = BTreeSet::new();
            for cell in &variant.cells {
                for &next in &cell.next_cells {
                    has_incoming.insert(next);
                }
            }
            for cell_no in &cell_nos {
                if *cell_no != 0 && !has_incoming.contains(cell_no) {
                    warnings.push(MapValidationWarning::Unreachable {
                        map_id,
                        cell_no: *cell_no,
                        variant: vkey.clone(),
                    });
                }
            }

            // Routing-rule targets not in next_cells
            for (&from_cell_no, rules) in &variant.routing_rules {
                let Some(next) = next_map.get(&from_cell_no) else {
                    continue;
                };
                for rule in rules {
                    if !next.contains(&rule.to_cell_no) {
                        warnings.push(MapValidationWarning::RuleTargetNotInNextCells {
                            map_id,
                            from_cell_no,
                            to_cell_no: rule.to_cell_no,
                            variant: vkey.clone(),
                        });
                    }
                }
            }
        }

        warnings
    }

    pub fn default_stage_id(&self) -> Option<&str> {
        if !self.default_variant.is_empty() && self.variants.contains_key(&self.default_variant) {
            return Some(&self.default_variant);
        }
        if self.variants.contains_key("") {
            return Some("");
        }
        self.variants.keys().next().map(String::as_str)
    }

    pub fn resolve_stage_id_for_rank(&self, rank: i64) -> Option<&str> {
        self.rank_stage_ids.get(&rank).map(String::as_str).or_else(|| self.default_stage_id())
    }

    pub fn active_stage(&self, stage_id: Option<&str>) -> Option<&MapStageDefinition> {
        stage_id
            .and_then(|stage_id| self.variants.get(stage_id))
            .or_else(|| self.default_stage_id().and_then(|stage_id| self.variants.get(stage_id)))
    }

    pub fn stage_for_rank(&self, rank: i64) -> Option<&MapStageDefinition> {
        self.resolve_stage_id_for_rank(rank).and_then(|stage_id| self.active_stage(Some(stage_id)))
    }

    pub fn stage(&self, stage_id: &str) -> Option<&MapStageDefinition> {
        self.active_stage(Some(stage_id))
    }

    pub fn variant(&self, variant_key: &str) -> Option<&MapVariantDefinition> {
        self.variants
            .get(variant_key)
            .or_else(|| self.variants.get(&self.default_variant))
            .or_else(|| self.variants.get(""))
    }
}

impl MapVariantDefinition {
    pub fn cell(&self, cell_no: i64) -> Option<&MapCellDefinition> {
        self.cells.iter().find(|cell| cell.cell_no == cell_no)
    }

    pub fn ship_drops(&self, cell_no: i64) -> Option<&[ShipDropDefinition]> {
        self.ship_drops.get(&cell_no).map(Vec::as_slice)
    }

    pub fn first_progress_cell_no(&self) -> Option<i64> {
        if let Some(rules) = self.routing_rules.get(&0).filter(|rules| !rules.is_empty()) {
            let targets = rules.iter().map(|rule| rule.to_cell_no).collect::<BTreeSet<_>>();
            return (targets.len() == 1).then(|| targets.into_iter().next()).flatten();
        }
        if let Some(start) = self.cell(0) {
            return match start.next_cells.as_slice() {
                [only] => Some(*only),
                _ => None,
            };
        }
        let mut cells = self.cells.iter().filter(|cell| cell.cell_no > 0).map(|cell| cell.cell_no);
        let first = cells.next()?;
        cells.next().is_none().then_some(first)
    }
}

fn compose_map_id(maparea_id: i64, mapinfo_no: i64) -> i64 {
    maparea_id * 10 + mapinfo_no
}

/// Build the regular (non-event) map prerequisite table.
///
/// Rules:
/// - 1-1 has no prerequisite (always unlocked)
/// - Same area sequential: N-M requires N-(M-1) cleared
/// - Cross-area: clearing area boss (N-4) unlocks (N+1)-1
/// - EO maps (N-5, N-6, ...) require the preceding map in the same area
pub(crate) fn build_regular_prerequisites() -> HashMap<i64, i64> {
    let mut prereqs = HashMap::new();

    // Same-area sequential: N-2 requires N-1, N-3 requires N-2, ..., including EO maps
    for area in 1..=7 {
        for no in 2..=9 {
            prereqs.insert(compose_map_id(area, no), compose_map_id(area, no - 1));
        }
    }

    // Cross-area: area N boss (N-4) cleared unlocks (N+1)-1
    for area in 1..=6 {
        prereqs.insert(compose_map_id(area + 1, 1), compose_map_id(area, 4));
    }

    prereqs
}

pub fn split_map_id(map_id: i64) -> (i64, i64) {
    (map_id / 10, map_id % 10)
}

pub fn extract_max_hp(map: &ApiMstMapinfo) -> Option<i64> {
    match map.api_max_maphp.as_ref()? {
        serde_json::Value::Number(value) => value.as_i64(),
        serde_json::Value::String(value) => value.parse::<i64>().ok(),
        serde_json::Value::Array(values) => values.first().and_then(|value| match value {
            serde_json::Value::Number(number) => number.as_i64(),
            serde_json::Value::String(string) => string.parse::<i64>().ok(),
            _ => None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_map_prerequisites() {
        let prereqs = build_regular_prerequisites();
        // 1-2 requires 1-1
        assert_eq!(prereqs.get(&12), Some(&11));
        // 2-1 requires 1-4
        assert_eq!(prereqs.get(&21), Some(&14));
        // 1-1 has no prerequisite
        assert_eq!(prereqs.get(&11), None);
    }

    #[test]
    fn eo_map_prerequisites() {
        let prereqs = build_regular_prerequisites();
        // 1-5 (EO) requires 1-4
        assert_eq!(prereqs.get(&15), Some(&14));
        // 1-6 requires 1-5
        assert_eq!(prereqs.get(&16), Some(&15));
        // 2-5 requires 2-4
        assert_eq!(prereqs.get(&25), Some(&24));
        // 3-5 requires 3-4
        assert_eq!(prereqs.get(&35), Some(&34));
        // 7-5 requires 7-4
        assert_eq!(prereqs.get(&75), Some(&74));
    }

    #[test]
    fn all_areas_have_eo_chains() {
        let prereqs = build_regular_prerequisites();
        for area in 1..=7 {
            // Each area should have prerequisites for maps 2-9
            for no in 2..=9 {
                let map_id = compose_map_id(area, no);
                assert!(
                    prereqs.contains_key(&map_id),
                    "area {area} map no {no} (map_id={map_id}) should have a prerequisite"
                );
            }
        }
    }

    // ------------------------------------------------------------------ validation (U7)

    fn valid_map_definition() -> MapDefinition {
        MapDefinition {
            map_id: 11,
            maparea_id: 1,
            mapinfo_no: 1,
            name: "test".into(),
            level: 1,
            sally_flag: vec![],
            is_event: false,
            reset_policy: MapResetPolicy::Never,
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
                        MapCellDefinition {
                            cell_no: 0,
                            next_cells: vec![1, 2],
                            ..Default::default()
                        },
                        MapCellDefinition {
                            cell_no: 1,
                            next_cells: vec![3],
                            ..Default::default()
                        },
                        MapCellDefinition {
                            cell_no: 2,
                            next_cells: vec![3],
                            ..Default::default()
                        },
                        MapCellDefinition {
                            cell_no: 3,
                            next_cells: vec![],
                            ..Default::default()
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

    #[test]
    fn valid_map_produces_no_warnings() {
        let def = valid_map_definition();
        assert!(def.validate().is_empty());
    }

    #[test]
    fn self_loop_detected() {
        let mut def = valid_map_definition();
        let variant = def.variants.get_mut("").unwrap();
        // Make cell 1 point to itself
        variant.cells[1].next_cells = vec![1, 3];

        let warnings = def.validate();
        assert!(warnings.iter().any(|w| matches!(
            w,
            MapValidationWarning::SelfLoop {
                cell_no: 1,
                ..
            }
        )));
    }

    #[test]
    fn unreachable_cell_detected() {
        let mut def = valid_map_definition();
        // Add cell 99 with no incoming edges
        let variant = def.variants.get_mut("").unwrap();
        variant.cells.push(MapCellDefinition {
            cell_no: 99,
            next_cells: vec![],
            ..Default::default()
        });

        let warnings = def.validate();
        assert!(warnings.iter().any(|w| matches!(
            w,
            MapValidationWarning::Unreachable {
                cell_no: 99,
                ..
            }
        )));
    }

    #[test]
    fn rule_target_not_in_next_cells() {
        let mut def = valid_map_definition();
        let variant = def.variants.get_mut("").unwrap();
        variant.routing_rules.insert(
            0,
            vec![RouteRule {
                from_cell_no: 0,
                to_cell_no: 99,
                priority: 1,
                weight: None,
                probability_pct: None,
                predicate: RoutePredicate::Always,
                raw_text: String::new(),
            }],
        );

        let warnings = def.validate();
        assert!(warnings.iter().any(|w| matches!(
            w,
            MapValidationWarning::RuleTargetNotInNextCells {
                from_cell_no: 0,
                to_cell_no: 99,
                ..
            }
        )));
    }

    #[test]
    fn synthetic_variant_boss_sentinel_is_zero() {
        let mut catalog = MapCatalog {
            maps: BTreeMap::new(),
            prerequisites: HashMap::new(),
        };
        // Insert a map with no variants
        catalog.maps.insert(
            99,
            MapDefinition {
                map_id: 99,
                maparea_id: 9,
                mapinfo_no: 9,
                name: "empty".into(),
                level: 1,
                sally_flag: vec![],
                is_event: false,
                reset_policy: MapResetPolicy::Never,
                airbase_count: None,
                gauge_type: None,
                gauge_count: None,
                required_defeat_count: None,
                max_hp: None,
                default_variant: String::new(),
                rank_stage_ids: BTreeMap::new(),
                variants: BTreeMap::new(),
            },
        );
        catalog.ensure_synthetic_variants();

        let variant = catalog.maps[&99].variants.get("").unwrap();
        assert_eq!(variant.boss_cell_no, 0, "synthetic variant must use boss_cell_no = 0 sentinel");
    }
}
