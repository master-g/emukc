//! Structural / topological validator for decoded map route definitions (U5, R5, KTD5).
//!
//! This mirrors [`crate::battle_rules`]'s validator/finding/report/severity shape but
//! operates over the **public** map route types from `emukc_model`
//! ([`MapVariantDefinition`] / [`MapStageDefinition`], [`MapCellDefinition`], [`RouteRule`],
//! [`RoutePredicate`]). It catches **structural corruption** the way the battle validators
//! catch protocol drift: route edges that point off the topology, missing cells, and
//! unsupported predicates slipping into the route rules.
//!
//! Per **KTD5 this is STRUCTURAL, not SEMANTIC** validation. It does **not** detect a
//! predicate whose threshold is wrong (e.g. `FleetSize >= 4` vs the client's `>= 5`) — that
//! threshold comes from the same wikiwiki source being validated, so checking it against
//! itself is circular. It also does **not** assert a deterministic next-cell, because routing
//! legitimately uses weighted random.
//!
//! `emukc_bootstrap` must not depend on `emukc_gameplay` (it would be a dependency cycle), so
//! this validator cannot call the production router. It re-checks only the declared topology.
//! Behavioral edge-legality (driving the real `evaluate_route_destination` over a fleet-config
//! matrix and asserting every returned cell is a declared `next_cell`) lives in an in-crate
//! `#[cfg(test)]` module in `crates/emukc_gameplay/src/game/map_route.rs`, which is the only
//! place that can reach the router's `pub(crate)` surface.
#![allow(missing_docs)]

use std::collections::BTreeSet;

use emukc_model::codex::map::{MapStageDefinition, RoutePredicate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapRouteValidationSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapRouteValidationFindingKind {
    /// A `routing_rules` rule targets a cell absent from the departing cell's `next_cells`
    /// (the edge would route off the declared topology).
    RuleTargetNotInNextCells,
    /// A `next_cells` entry references a cell number that is not a real cell of the stage.
    NextCellNotInStage,
    /// A `routing_rules` key (the `from` cell) is not a real cell of the stage.
    RuleFromCellNotInStage,
    /// A route rule carries an unsupported predicate (`Unknown` / `SourceUnknown`); a live
    /// routing decision over it would silently fall through.
    UnsupportedPredicate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapRouteValidationFinding {
    pub severity: MapRouteValidationSeverity,
    pub kind: MapRouteValidationFindingKind,
    /// The cell the finding originates from (the `from` cell of an edge / rule).
    pub from_cell_no: i64,
    /// The cell the finding points at (a rule's `to_cell_no` or a `next_cells` entry), where
    /// applicable.
    pub to_cell_no: Option<i64>,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapRouteValidationReport {
    pub findings: Vec<MapRouteValidationFinding>,
}

impl MapRouteValidationReport {
    pub fn has_errors(&self) -> bool {
        self.findings.iter().any(|finding| finding.severity == MapRouteValidationSeverity::Error)
    }

    fn push_error(
        &mut self,
        kind: MapRouteValidationFindingKind,
        from_cell_no: i64,
        to_cell_no: Option<i64>,
        message: impl Into<String>,
    ) {
        self.findings.push(MapRouteValidationFinding {
            severity: MapRouteValidationSeverity::Error,
            kind,
            from_cell_no,
            to_cell_no,
            message: message.into(),
        });
    }

    fn push_warning(
        &mut self,
        kind: MapRouteValidationFindingKind,
        from_cell_no: i64,
        to_cell_no: Option<i64>,
        message: impl Into<String>,
    ) {
        self.findings.push(MapRouteValidationFinding {
            severity: MapRouteValidationSeverity::Warning,
            kind,
            from_cell_no,
            to_cell_no,
            message: message.into(),
        });
    }
}

/// Walk a `RoutePredicate` tree and flag any unsupported leaf (`Unknown` / `SourceUnknown`).
///
/// `And`/`Or`/`Not` are recursed; everything else is a supported leaf. The walk does **not**
/// evaluate the predicate (no fleet context, no semantics) — it only inspects which predicate
/// kinds the decoded data carries, which is the only structural question that can be answered
/// without a fleet matrix and without re-implementing the gameplay matcher.
fn flag_unsupported_predicates(
    predicate: &RoutePredicate,
    from_cell_no: i64,
    to_cell_no: i64,
    report: &mut MapRouteValidationReport,
) {
    match predicate {
        RoutePredicate::Unknown {
            raw_text,
        } => {
            report.push_warning(
                MapRouteValidationFindingKind::UnsupportedPredicate,
                from_cell_no,
                Some(to_cell_no),
                format!(
                    "rule {from_cell_no}->{to_cell_no} uses an Unknown predicate (raw: {raw_text:?}); a live decision over it falls through"
                ),
            );
        }
        RoutePredicate::SourceUnknown {
            raw_text,
        } => {
            report.push_warning(
                MapRouteValidationFindingKind::UnsupportedPredicate,
                from_cell_no,
                Some(to_cell_no),
                format!(
                    "rule {from_cell_no}->{to_cell_no} uses a SourceUnknown predicate (raw: {raw_text:?}); a live decision over it falls through"
                ),
            );
        }
        RoutePredicate::And(predicates) | RoutePredicate::Or(predicates) => {
            for inner in predicates {
                flag_unsupported_predicates(inner, from_cell_no, to_cell_no, report);
            }
        }
        RoutePredicate::Not(inner) => {
            flag_unsupported_predicates(inner, from_cell_no, to_cell_no, report);
        }
        _ => {}
    }
}

/// Validate one map stage (a [`MapStageDefinition`] / [`MapVariantDefinition`]) for the
/// structural KTD5 invariants:
///
/// - (a) **Topology** (errors): every `next_cells` entry references a real cell of the stage;
///   every `routing_rules` key references a real cell; every routing-rule `to_cell_no` is
///   present in the departing cell's `next_cells` (no edge points off the topology).
/// - (b) **Predicate support** (warnings): every route rule's predicate tree is free of
///   `Unknown` / `SourceUnknown` leaves, so an unsupported predicate slipping into a live
///   decision is visible.
///
/// It does **not** check predicate thresholds (semantic; circular against the wiki source) and
/// it does **not** assert deterministic destinations (routing uses weighted random) — see KTD5.
///
/// Note on (c) (a fleet-config matrix asserting a predicate that *should* match a config
/// evaluates without being `Unsupported`): expressing that purely over `emukc_model` types
/// would mean duplicating the gameplay predicate evaluator (`route_predicate_matches`), which
/// lives `pub(crate)` in `emukc_gameplay`. That is out of scope here per KTD5 (semantic eval).
/// The behavioral fleet-config matrix instead drives the *real* router from an in-crate
/// `emukc_gameplay` test; this validator stays a pure structural linter.
pub fn validate_map_route_stage(stage: &MapStageDefinition) -> MapRouteValidationReport {
    let mut report = MapRouteValidationReport::default();

    let cell_nos: BTreeSet<i64> = stage.cells.iter().map(|cell| cell.cell_no).collect();

    // (a) next_cells must point at real cells of the stage.
    for cell in &stage.cells {
        for &next in &cell.next_cells {
            if !cell_nos.contains(&next) {
                report.push_error(
                    MapRouteValidationFindingKind::NextCellNotInStage,
                    cell.cell_no,
                    Some(next),
                    format!(
                        "cell {} lists next_cell {next} that is not a real cell of the stage",
                        cell.cell_no
                    ),
                );
            }
        }
    }

    // (a) routing_rules: the from-cell must exist, and each rule's to-cell must be in that
    // from-cell's next_cells (the edge must be on the declared topology).
    for (&from_cell_no, rules) in &stage.routing_rules {
        let next_cells: BTreeSet<i64> = match stage.cell(from_cell_no) {
            Some(cell) => cell.next_cells.iter().copied().collect(),
            None => {
                report.push_error(
                    MapRouteValidationFindingKind::RuleFromCellNotInStage,
                    from_cell_no,
                    None,
                    format!(
                        "routing_rules reference from-cell {from_cell_no} that is not a real cell of the stage"
                    ),
                );
                BTreeSet::new()
            }
        };

        for rule in rules {
            if !next_cells.contains(&rule.to_cell_no) {
                report.push_error(
                    MapRouteValidationFindingKind::RuleTargetNotInNextCells,
                    from_cell_no,
                    Some(rule.to_cell_no),
                    format!(
                        "cell {from_cell_no} routing rule targets {} outside its next_cells {next_cells:?}",
                        rule.to_cell_no
                    ),
                );
            }

            // (b) predicate support
            flag_unsupported_predicates(
                &rule.predicate,
                from_cell_no,
                rule.to_cell_no,
                &mut report,
            );
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use emukc_model::codex::map::{MapCellDefinition, MapVariantDefinition, RouteRule};
    use std::collections::BTreeMap;

    fn cell(cell_no: i64, next_cells: Vec<i64>) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            next_cells,
            ..Default::default()
        }
    }

    fn always_rule(from: i64, to: i64) -> RouteRule {
        RouteRule {
            from_cell_no: from,
            to_cell_no: to,
            priority: 0,
            predicate: RoutePredicate::Always,
            ..Default::default()
        }
    }

    /// Overwrite a cell's `next_cells` in place. `MapVariantDefinition` is a foreign type, so
    /// this is a free helper rather than an inherent method.
    fn set_next_cells(stage: &mut MapVariantDefinition, cell_no: i64, next_cells: Vec<i64>) {
        let cell = stage.cells.iter_mut().find(|c| c.cell_no == cell_no).expect("cell exists");
        cell.next_cells = next_cells;
    }

    /// Clean synthetic stage: 0 -> {1,2}; 1 -> {3}; 2 -> {3}; 3 -> {}. A routing rule on
    /// cell 0 splits to both children, each target listed in cell 0's `next_cells`.
    fn clean_stage() -> MapVariantDefinition {
        let mut routing_rules = BTreeMap::new();
        routing_rules.insert(0, vec![always_rule(0, 1), always_rule(0, 2)]);
        MapVariantDefinition {
            boss_cell_no: 3,
            cells: vec![cell(0, vec![1, 2]), cell(1, vec![3]), cell(2, vec![3]), cell(3, vec![])],
            routing_rules,
            ..Default::default()
        }
    }

    #[test]
    fn clean_stage_has_no_violations() {
        let report = validate_map_route_stage(&clean_stage());
        assert!(report.findings.is_empty(), "clean stage produced findings: {report:?}");
        assert!(!report.has_errors());
    }

    #[test]
    fn rule_target_absent_from_next_cells_is_flagged() {
        let mut stage = clean_stage();
        // Point cell 0's rule at cell 2, but drop 2 from cell 0's next_cells so the edge
        // is off the topology.
        set_next_cells(&mut stage, 0, vec![1]);
        let report = validate_map_route_stage(&stage);
        assert!(report.has_errors());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == MapRouteValidationFindingKind::RuleTargetNotInNextCells
                    && f.from_cell_no == 0
                    && f.to_cell_no == Some(2)),
            "expected RuleTargetNotInNextCells for 0->2, got {report:?}"
        );
    }

    #[test]
    fn next_cell_not_in_stage_is_flagged() {
        let mut stage = clean_stage();
        // Cell 1 now points at a non-existent cell 99.
        set_next_cells(&mut stage, 1, vec![3, 99]);
        let report = validate_map_route_stage(&stage);
        assert!(report.has_errors());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == MapRouteValidationFindingKind::NextCellNotInStage
                    && f.from_cell_no == 1
                    && f.to_cell_no == Some(99)),
            "expected NextCellNotInStage for cell 1 -> 99, got {report:?}"
        );
    }

    #[test]
    fn rule_from_cell_not_in_stage_is_flagged() {
        let mut stage = clean_stage();
        // Add routing rules keyed on a cell number that doesn't exist.
        stage.routing_rules.insert(42, vec![always_rule(42, 1)]);
        let report = validate_map_route_stage(&stage);
        assert!(report.has_errors());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == MapRouteValidationFindingKind::RuleFromCellNotInStage
                    && f.from_cell_no == 42),
            "expected RuleFromCellNotInStage for cell 42, got {report:?}"
        );
    }

    #[test]
    fn unknown_predicate_is_flagged_as_warning() {
        let mut stage = clean_stage();
        // Replace cell 0's first rule predicate with Unknown.
        stage.routing_rules.get_mut(&0).unwrap()[0].predicate = RoutePredicate::Unknown {
            raw_text: "???".to_string(),
        };
        let report = validate_map_route_stage(&stage);
        // Unsupported predicates are warnings, not hard errors (the wiki source is
        // known-incomplete), but must still surface.
        assert!(!report.has_errors(), "unsupported predicate must not be a hard error: {report:?}");
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == MapRouteValidationFindingKind::UnsupportedPredicate
                    && f.severity == MapRouteValidationSeverity::Warning
                    && f.from_cell_no == 0),
            "expected UnsupportedPredicate warning, got {report:?}"
        );
    }

    #[test]
    fn source_unknown_predicate_is_flagged_as_warning() {
        let mut stage = clean_stage();
        stage.routing_rules.get_mut(&0).unwrap()[1].predicate = RoutePredicate::SourceUnknown {
            raw_text: "ambiguous wiki text".to_string(),
        };
        let report = validate_map_route_stage(&stage);
        assert!(!report.has_errors());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == MapRouteValidationFindingKind::UnsupportedPredicate
                    && f.severity == MapRouteValidationSeverity::Warning
                    && f.to_cell_no == Some(2)),
            "expected UnsupportedPredicate warning for rule 0->2, got {report:?}"
        );
    }

    #[test]
    fn unsupported_predicate_nested_in_and_is_flagged() {
        let mut stage = clean_stage();
        // Nest a SourceUnknown leaf inside And/Not to prove the walk recurses.
        stage.routing_rules.get_mut(&0).unwrap()[0].predicate = RoutePredicate::And(vec![
            RoutePredicate::Always,
            RoutePredicate::Not(Box::new(RoutePredicate::SourceUnknown {
                raw_text: "nested".to_string(),
            })),
        ]);
        let report = validate_map_route_stage(&stage);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == MapRouteValidationFindingKind::UnsupportedPredicate),
            "expected nested unsupported predicate to be flagged, got {report:?}"
        );
    }
}
