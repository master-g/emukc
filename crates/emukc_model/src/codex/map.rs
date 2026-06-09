#![allow(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashMap};

mod debug;
mod merge;
mod types;

#[expect(deprecated)]
use crate::profile::map_record::DEFAULT_MAP_RECORDS;
use crate::{
    kc2::start2::{ApiManifest, ApiMstMapinfo},
    profile::map_record::MapRefreshType,
};

pub use types::*;

use merge::merge_definition as merge_definition_impl;
pub use merge::{build_cell_no_map, merge_routing_overlay};

/// P-unlock variant keys, in canonical (pre → post) order. A map carrying any of these is a
/// P-unlock map whose route is gated by a sub-gauge unlock.
const P_UNLOCK_VARIANT_KEYS: [&str; 2] = ["pre_p_unlock", "post_p_unlock"];

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
        #[expect(deprecated)]
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
    /// Returns warnings for: self-loops, unreachable cells, routing-rule targets not in `next_cells`.
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

    /// Normalize a P-unlock map so its `pre_p_unlock` / `post_p_unlock` variants are the
    /// canonical variant set.
    ///
    /// P-unlock variants typically arrive as topology-less skeletons (from public-overlay
    /// captures): they carry the per-phase cell set but no `next_cells` / routing, while the
    /// real route graph lives only in the base `""` variant. The 3-source merge keeps that
    /// `""` variant as the default and derives `gauge_count` from the kcdata side, leaving the
    /// P-unlock variants unroutable (no start cell). This pass:
    ///
    /// 1. folds the base `""` topology into each P-unlock variant, **restricted to that
    ///    variant's own cells** so `pre_p_unlock` stays a strict subgraph (post-unlock cells
    ///    are unreachable before unlocking);
    /// 2. makes `pre_p_unlock` the default and derives `gauge_count` from the P-unlock set;
    /// 3. wires the `pre → post` progression (`clear_to_variant_key`, `required_defeat_count`)
    ///    the capture path leaves unset;
    /// 4. drops the now-spurious empty-key `""` variant.
    ///
    /// No-op for maps without P-unlock variants. Degrades safely (skips the fold) when the
    /// base `""` topology is absent, and never overwrites topology a variant already carries.
    pub fn normalize_p_unlock_variants(&mut self) {
        let [pre_key, post_key] = P_UNLOCK_VARIANT_KEYS;
        let present: Vec<&'static str> = P_UNLOCK_VARIANT_KEYS
            .iter()
            .copied()
            .filter(|key| self.variants.contains_key(*key))
            .collect();
        if present.is_empty() {
            return;
        }

        // Fold the base "" topology into each p_unlock skeleton (restricted to its cells),
        // then drop it: the p_unlock set is canonical now. Taking ownership via `remove`
        // avoids cloning the whole base variant and makes the drop explicit.
        if let Some(base) = self.variants.remove("") {
            for key in &present {
                if let Some(variant) = self.variants.get_mut(*key) {
                    fold_base_topology_into_variant(variant, &base);
                }
            }

            // Routability guard: the fold joins variant ↔ base by `cell_no`. If a p_unlock
            // variant's capture numbering diverges from the kcdata base, the fold copies
            // nothing and the would-be default ends up with no start cell. Rather than destroy
            // the only routable variant, abort normalization and keep the kcdata base as the
            // playable default — the p_unlock split is deferred until the data aligns.
            let default_routable =
                self.variants.get(present[0]).is_some_and(|variant| variant.has_start_source());
            if !default_routable {
                self.variants.insert(String::new(), base);
                tracing::warn!(
                    map_id = self.map_id,
                    default_variant = present[0],
                    "p_unlock topology fold left the default variant unroutable; keeping kcdata base"
                );
                return;
            }
        }

        // `pre_p_unlock` is the pre-unlock phase and the canonical default; fall back to the
        // first present p_unlock variant if `pre_p_unlock` is somehow absent.
        self.default_variant = present[0].to_string();

        // The remaining derivations apply only to the canonical pre + post pair. A lone
        // p_unlock variant (a partial fixture or incomplete capture) keeps its upstream
        // gauge_count rather than being clobbered to 1, and needs no progression wiring.
        if present.len() >= 2 {
            self.gauge_count = Some(present.len() as i64);

            // Wire the pre → post progression the capture path leaves unset.
            let map_required_defeat_count = self.required_defeat_count;
            if let Some(pre) = self.variants.get_mut(pre_key) {
                pre.clear_to_variant_key.get_or_insert_with(|| post_key.to_string());
                if pre.required_defeat_count.is_none() {
                    pre.required_defeat_count = map_required_defeat_count;
                }
            }
        }
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

    /// Returns `true` when `cell_no` has any outgoing connection in this variant —
    /// either a non-empty `next_cells` on the cell itself, or at least one routing rule
    /// keyed on it.
    ///
    /// This does **not** check whether a rule's `to_cell_no` is listed in the cell's
    /// `next_cells`; a cell can carry routing rules whose targets are disjoint from its
    /// topology-level `next_cells`. Callers needing that cross-check must do it separately.
    pub fn cell_has_routing_outgoing(&self, cell_no: i64) -> bool {
        self.cell(cell_no).is_some_and(|cell| !cell.next_cells.is_empty())
            || self.routing_rules.get(&cell_no).is_some_and(|rules| !rules.is_empty())
    }

    /// Resolve the cell(s) a sortie can start from: the graph roots (cells with no incoming
    /// edge that have outgoing routing); falling back to cell 0 when it has outgoing routing.
    /// Empty when the variant has no routable start (e.g. a topology-less skeleton).
    pub fn start_source_cells(&self) -> Vec<&MapCellDefinition> {
        let incoming = self
            .cells
            .iter()
            .flat_map(|cell| cell.next_cells.iter().copied())
            .collect::<BTreeSet<_>>();
        let roots = self
            .cells
            .iter()
            .filter(|cell| {
                !incoming.contains(&cell.cell_no) && self.cell_has_routing_outgoing(cell.cell_no)
            })
            .collect::<Vec<_>>();
        if !roots.is_empty() {
            return roots;
        }

        self.cell(0)
            .filter(|cell| self.cell_has_routing_outgoing(cell.cell_no))
            .into_iter()
            .collect()
    }

    /// Whether a sortie can resolve a start cell for this variant. Boolean form of
    /// [`start_source_cells`](Self::start_source_cells), used by catalog assembly to guard
    /// destructive P-unlock normalization against an unroutable topology fold.
    pub fn has_start_source(&self) -> bool {
        !self.start_source_cells().is_empty()
    }
}

/// Fill a P-unlock variant's missing `next_cells` / routing from the base `""` variant,
/// **restricted to the variant's own cell set** so post-unlock cells stay unreachable in the
/// pre-unlock phase. Cells are joined by their shared in-game cell number (kcdata and the public
/// capture use the same numbering). Topology a variant already carries is preserved.
fn fold_base_topology_into_variant(
    variant: &mut MapVariantDefinition,
    base: &MapVariantDefinition,
) {
    let cell_set: BTreeSet<i64> = variant.cells.iter().map(|cell| cell.cell_no).collect();
    let base_by_no: BTreeMap<i64, &MapCellDefinition> =
        base.cells.iter().map(|cell| (cell.cell_no, cell)).collect();

    for cell in &mut variant.cells {
        let Some(base_cell) = base_by_no.get(&cell.cell_no) else {
            continue;
        };
        if cell.next_cells.is_empty() {
            cell.next_cells = base_cell
                .next_cells
                .iter()
                .copied()
                .filter(|next| cell_set.contains(next))
                .collect();
        }
        if cell.node_label.is_none() {
            cell.node_label = base_cell.node_label.clone();
        }
        if cell.distance.is_none() {
            cell.distance = base_cell.distance;
        }
    }

    if variant.routing_rules.is_empty() {
        for (from_cell_no, rules) in &base.routing_rules {
            if !cell_set.contains(from_cell_no) {
                continue;
            }
            let kept: Vec<RouteRule> =
                rules.iter().filter(|rule| cell_set.contains(&rule.to_cell_no)).cloned().collect();
            if !kept.is_empty() {
                variant.routing_rules.insert(*from_cell_no, kept);
            }
        }
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

    // ── normalize_p_unlock_variants (U2) ─────────────────────────────────────

    fn labeled_cell(cell_no: i64, next_cells: Vec<i64>) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            next_cells,
            node_label: Some(format!("N{cell_no}")),
            ..Default::default()
        }
    }

    /// A cell as produced by the public overlay capture: a known cell number but no outgoing
    /// topology and no label.
    fn skeleton_cell(cell_no: i64) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            master_cell_id: Some(1000 + cell_no),
            ..Default::default()
        }
    }

    /// Mirror of map 73: base `""` carries the full route graph; pre/post are topology-less
    /// skeletons whose cell *sets* encode per-phase membership (pre ⊂ post == base).
    fn p_unlock_definition() -> MapDefinition {
        let base = MapVariantDefinition {
            variant_key: String::new(),
            boss_cell_no: 18,
            cells: vec![
                labeled_cell(0, vec![1]),
                labeled_cell(1, vec![2, 3]),
                labeled_cell(2, vec![3]),
                labeled_cell(3, vec![]),
            ],
            ..Default::default()
        };
        let pre = MapVariantDefinition {
            variant_key: "pre_p_unlock".to_string(),
            boss_cell_no: 2,
            cells: vec![skeleton_cell(0), skeleton_cell(1), skeleton_cell(2)],
            ..Default::default()
        };
        let post = MapVariantDefinition {
            variant_key: "post_p_unlock".to_string(),
            boss_cell_no: 3,
            cells: vec![skeleton_cell(0), skeleton_cell(1), skeleton_cell(2), skeleton_cell(3)],
            ..Default::default()
        };
        MapDefinition {
            map_id: 73,
            maparea_id: 7,
            mapinfo_no: 3,
            name: "7-3".into(),
            level: 1,
            sally_flag: vec![],
            is_event: false,
            reset_policy: MapResetPolicy::Never,
            airbase_count: None,
            gauge_type: Some(1),
            gauge_count: Some(1),
            required_defeat_count: Some(3),
            max_hp: None,
            default_variant: String::new(),
            rank_stage_ids: BTreeMap::new(),
            variants: BTreeMap::from([
                (String::new(), base),
                ("pre_p_unlock".to_string(), pre),
                ("post_p_unlock".to_string(), post),
            ]),
        }
    }

    #[test]
    fn normalize_p_unlock_sets_default_gauge_and_drops_empty_variant() {
        let mut def = p_unlock_definition();
        def.normalize_p_unlock_variants();

        assert_eq!(def.default_variant, "pre_p_unlock");
        assert_eq!(def.gauge_count, Some(2));
        let keys: Vec<&str> = def.variants.keys().map(String::as_str).collect();
        // BTreeMap order; the spurious "" variant is gone.
        assert_eq!(keys, vec!["post_p_unlock", "pre_p_unlock"]);
    }

    #[test]
    fn normalize_p_unlock_folds_base_topology_restricted_to_variant_cells() {
        let mut def = p_unlock_definition();
        def.normalize_p_unlock_variants();

        // pre keeps only cells {0,1,2}; base edge 1→3 is dropped (3 ∉ pre), 1→2 kept.
        let pre = def.variants.get("pre_p_unlock").unwrap();
        assert_eq!(pre.cell(0).unwrap().next_cells, vec![1]);
        assert_eq!(pre.cell(1).unwrap().next_cells, vec![2]);
        assert!(pre.cell(2).unwrap().next_cells.is_empty());

        // post spans every base cell, so it receives the full base topology.
        let post = def.variants.get("post_p_unlock").unwrap();
        assert_eq!(post.cell(0).unwrap().next_cells, vec![1]);
        assert_eq!(post.cell(1).unwrap().next_cells, vec![2, 3]);
    }

    #[test]
    fn normalize_p_unlock_wires_pre_to_post_progression() {
        let mut def = p_unlock_definition();
        def.normalize_p_unlock_variants();

        let pre = def.variants.get("pre_p_unlock").unwrap();
        assert_eq!(pre.clear_to_variant_key.as_deref(), Some("post_p_unlock"));
        assert_eq!(pre.required_defeat_count, Some(3));
    }

    #[test]
    fn normalize_non_p_unlock_map_is_unchanged() {
        let mut def = valid_map_definition();
        let before = def.clone();
        def.normalize_p_unlock_variants();
        assert_eq!(def.default_variant, before.default_variant);
        assert_eq!(def.gauge_count, before.gauge_count);
        assert_eq!(
            def.variants.keys().collect::<Vec<_>>(),
            before.variants.keys().collect::<Vec<_>>()
        );
        // The fold must never touch a non-p_unlock map's topology.
        assert_eq!(def.variants[""].cells, before.variants[""].cells);
    }

    #[test]
    fn normalize_p_unlock_folds_routing_rules_restricted_to_variant_cells() {
        let mut def = p_unlock_definition();
        // Base "" routing: 1→2 (within pre) and 1→3 (post-only). pre lacks cell 3.
        def.variants.get_mut("").unwrap().routing_rules.insert(
            1,
            vec![
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 2,
                    priority: 0,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Always,
                    raw_text: String::new(),
                },
                RouteRule {
                    from_cell_no: 1,
                    to_cell_no: 3,
                    priority: 1,
                    weight: None,
                    probability_pct: None,
                    predicate: RoutePredicate::Always,
                    raw_text: String::new(),
                },
            ],
        );
        def.normalize_p_unlock_variants();

        // pre {0,1,2}: only the 1→2 rule survives (target 3 is outside pre).
        let pre = def.variants.get("pre_p_unlock").unwrap();
        let pre_rules = pre.routing_rules.get(&1).expect("pre keeps the in-set rule");
        assert_eq!(pre_rules.len(), 1);
        assert_eq!(pre_rules[0].to_cell_no, 2);

        // post spans every base cell, so both rules survive.
        assert_eq!(
            def.variants.get("post_p_unlock").unwrap().routing_rules.get(&1).unwrap().len(),
            2
        );
    }

    #[test]
    fn normalize_lone_pre_p_unlock_preserves_upstream_gauge_count() {
        let mut def = p_unlock_definition();
        def.variants.remove("post_p_unlock"); // only pre present
        def.normalize_p_unlock_variants();

        assert_eq!(def.default_variant, "pre_p_unlock");
        // Fixture gauge_count is Some(1); with no post pair it must not be clobbered to 1-derived.
        assert_eq!(def.gauge_count, Some(1), "lone p_unlock keeps upstream gauge_count");
        // No post to advance to → progression fields stay unset.
        assert_eq!(def.variants.get("pre_p_unlock").unwrap().clear_to_variant_key, None);
    }

    #[test]
    fn normalize_p_unlock_is_idempotent() {
        let mut once = p_unlock_definition();
        once.normalize_p_unlock_variants();

        let mut twice = p_unlock_definition();
        twice.normalize_p_unlock_variants();
        twice.normalize_p_unlock_variants();

        assert_eq!(once.default_variant, twice.default_variant);
        assert_eq!(once.gauge_count, twice.gauge_count);
        assert_eq!(
            once.variants.keys().collect::<Vec<_>>(),
            twice.variants.keys().collect::<Vec<_>>()
        );
        for key in once.variants.keys() {
            assert_eq!(once.variants[key].cells, twice.variants[key].cells, "cells diverge: {key}");
            assert_eq!(
                once.variants[key].clear_to_variant_key,
                twice.variants[key].clear_to_variant_key
            );
        }
    }

    #[test]
    fn normalize_p_unlock_without_base_degrades_safely() {
        let mut def = p_unlock_definition();
        def.variants.remove(""); // no base topology donor
        def.normalize_p_unlock_variants();

        assert_eq!(def.default_variant, "pre_p_unlock");
        assert_eq!(def.gauge_count, Some(2));
        // Nothing to fold → pre's start stays empty (no panic, best-effort).
        let pre = def.variants.get("pre_p_unlock").unwrap();
        assert!(pre.cell(0).unwrap().next_cells.is_empty());
    }

    #[test]
    fn normalize_p_unlock_preserves_existing_variant_topology() {
        let mut def = p_unlock_definition();
        // pre already has real topology (e.g. wikiwiki-native) — base must not clobber it.
        def.variants.get_mut("pre_p_unlock").unwrap().cells[0].next_cells = vec![2];
        def.normalize_p_unlock_variants();

        let pre = def.variants.get("pre_p_unlock").unwrap();
        assert_eq!(pre.cell(0).unwrap().next_cells, vec![2], "existing topology preserved");
    }

    /// Like `p_unlock_definition`, but the p_unlock skeletons are renumbered so they share NO
    /// cell_no with the base `""` graph — the topology fold (a cell_no join) matches nothing.
    fn misaligned_p_unlock_definition() -> MapDefinition {
        let mut def = p_unlock_definition();
        for key in ["pre_p_unlock", "post_p_unlock"] {
            let variant = def.variants.get_mut(key).unwrap();
            for cell in &mut variant.cells {
                cell.cell_no += 100;
            }
            variant.boss_cell_no += 100;
        }
        def
    }

    #[test]
    fn normalize_p_unlock_skips_when_fold_leaves_default_unroutable() {
        let mut def = misaligned_p_unlock_definition();
        let before_default = def.default_variant.clone();
        let before_gauge = def.gauge_count;
        def.normalize_p_unlock_variants();

        // Guard fired: the kcdata base is preserved; nothing destructive happened.
        assert!(def.variants.contains_key(""), "base \"\" variant must be retained");
        assert!(def.variants[""].has_start_source(), "retained base stays routable");
        assert_eq!(def.default_variant, before_default, "default not flipped to unroutable pre");
        assert_eq!(def.gauge_count, before_gauge, "gauge not re-derived on skip");
    }

    // ── start-source resolution (U1) ─────────────────────────────────────────

    fn variant_with(cells: Vec<MapCellDefinition>) -> MapVariantDefinition {
        MapVariantDefinition {
            cells,
            ..Default::default()
        }
    }

    #[test]
    fn has_start_source_true_for_cell0_with_next() {
        let variant = variant_with(vec![labeled_cell(0, vec![1]), labeled_cell(1, vec![])]);
        assert!(variant.has_start_source());
        let starts: Vec<i64> = variant.start_source_cells().iter().map(|c| c.cell_no).collect();
        assert_eq!(starts, vec![0]);
    }

    #[test]
    fn has_start_source_false_for_topologyless_skeleton() {
        let variant = variant_with(vec![skeleton_cell(0), skeleton_cell(1), skeleton_cell(2)]);
        assert!(!variant.has_start_source());
        assert!(variant.start_source_cells().is_empty());
    }

    #[test]
    fn start_source_cells_include_nonzero_root_with_outgoing() {
        // incoming = {13}; roots (no incoming, with outgoing) = {0 → 13, 22 → 13}.
        let variant = variant_with(vec![
            labeled_cell(0, vec![13]),
            labeled_cell(13, vec![]),
            labeled_cell(22, vec![13]),
        ]);
        let mut starts: Vec<i64> = variant.start_source_cells().iter().map(|c| c.cell_no).collect();
        starts.sort();
        assert_eq!(starts, vec![0, 22]);
    }

    #[test]
    fn cell_has_routing_outgoing_method_branches() {
        // next_cells only
        assert!(variant_with(vec![labeled_cell(1, vec![2])]).cell_has_routing_outgoing(1));

        // routing-rules only (empty next_cells but a rule keyed on the cell)
        let mut rule_only = variant_with(vec![skeleton_cell(1)]);
        rule_only.routing_rules.insert(
            1,
            vec![RouteRule {
                from_cell_no: 1,
                to_cell_no: 2,
                priority: 0,
                weight: None,
                probability_pct: None,
                predicate: RoutePredicate::Always,
                raw_text: String::new(),
            }],
        );
        assert!(rule_only.cell_has_routing_outgoing(1));

        // neither, and a missing cell
        let bare = variant_with(vec![skeleton_cell(1)]);
        assert!(!bare.cell_has_routing_outgoing(1));
        assert!(!bare.cell_has_routing_outgoing(99));
    }
}
