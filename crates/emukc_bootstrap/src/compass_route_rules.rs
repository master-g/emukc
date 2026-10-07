//! Routing rules converted from the compass simulator source.
//!
//! `main-decoder` translates the simulator's per-map branch functions into a neutral JSON
//! document. [`normalize_compass_route_rules`] turns that document's vocabulary into the
//! server's (ship types, variant keys, [`RoutePredicate`]s) and yields the checked-in asset;
//! [`apply_route_rules`] then pins the label-space rules onto a variant's cells.
//!
//! Nothing here guesses: a term, operator, phase or label without a known meaning is an
//! error, never an `Unknown` predicate.

use std::collections::BTreeMap;

use emukc_model::codex::map::{
    MapVariantDefinition, RouteCountTerm, RouteCounter, RouteOperator, RoutePredicate, RouteRule,
    SpeedClass,
};
use serde::{Deserialize, Serialize};

use crate::{assets::MAP_ROUTE_RULES, parser::error::ParseError};

/// Where the routing-rule asset lives in the repo.
pub fn repo_compass_route_rules_path() -> std::path::PathBuf {
    MAP_ROUTE_RULES.path()
}

/// Load the checked-in routing rules, falling back to the embedded copy.
pub(crate) fn load_repo_compass_route_rules() -> Result<CompassRouteRulesAsset, ParseError> {
    let path = repo_compass_route_rules_path();
    let (_, raw) = MAP_ROUTE_RULES.load().map_err(|source| ParseError::io_at(&path, source))?;
    serde_json::from_str(&raw).map_err(|source| ParseError::json_at(&path, source))
}

/// The checked-in routing rules, in label space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompassRouteRulesAsset {
    /// What the file is and how to regenerate it.
    pub note: String,
    /// Where the rules were converted from.
    pub source: CompassRouteRulesSource,
    /// Rules by map id, then by variant key.
    pub maps: BTreeMap<i64, BTreeMap<String, CompassVariantRouteRules>>,
}

/// Provenance of the converted rules, including the notice the MIT license asks to keep.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompassRouteRulesSource {
    /// `owner/name` of the source repository.
    pub repo: String,
    /// The converted commit.
    pub commit: String,
    /// First line of the source's `LICENSE`.
    pub license: String,
    /// Copyright line of the source's `LICENSE`.
    pub copyright: String,
}

/// One variant's rules, in the order the source tries them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompassVariantRouteRules {
    /// Rules choosing the start; empty when the map has one start. `to` is `1` or `2`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub start: Vec<LabelRouteRule>,
    /// Branching rules. `from` is a node label, or `1` / `2` for a start.
    pub rules: Vec<LabelRouteRule>,
}

/// A routing rule between two node labels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelRouteRule {
    /// Label of the node the fleet is on; empty on a start rule.
    pub from: String,
    /// Label of the node the rule leads to.
    pub to: String,
    /// Share of the roll this target takes when its rule fires with several targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub probability_pct: Option<f64>,
    /// When the rule fires.
    pub predicate: RoutePredicate,
}

#[derive(Debug, Deserialize)]
struct Document {
    source: CompassRouteRulesSource,
    maps: BTreeMap<String, BTreeMap<String, PhaseRules>>,
}

#[derive(Debug, Deserialize)]
struct PhaseRules {
    start: Vec<Rule>,
    nodes: BTreeMap<String, NodeRules>,
}

#[derive(Debug, Deserialize)]
struct NodeRules {
    rules: Vec<Rule>,
}

#[derive(Debug, Deserialize)]
struct Rule {
    cond: Option<Expr>,
    targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
struct Target {
    node: String,
    rate: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Expr {
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Not(Box<Expr>),
    Cmp {
        lhs: Vec<LhsTerm>,
        op: String,
        rhs: i64,
    },
    Los {
        cn: i64,
        op: String,
        value: f64,
    },
    Speed {
        op: String,
        value: i64,
    },
    Visited(String),
    FlagshipTypes(Vec<String>),
}

#[derive(Debug, Deserialize)]
struct LhsTerm {
    coef: i64,
    term: Term,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Term {
    ShipTypes {
        types: Vec<String>,
    },
    Ships {
        ids: Vec<i64>,
    },
    FleetSize,
    EquipCarriers {
        ids: Vec<i64>,
    },
    SlowShips {
        types: Vec<String>,
    },
}

/// The source's ship types as `api_stype` ids. It does not tell 巡洋戦艦 (8) from 戦艦 (9).
/// Checked against all 841 ships of the source's table at commit `4f32c40e`.
fn ship_type_ids(name: &str) -> Result<&'static [i64], String> {
    Ok(match name {
        "DE" => &[1],
        "DD" => &[2],
        "CL" => &[3],
        "CLT" => &[4],
        "CA" => &[5],
        "CAV" => &[6],
        "CVL" => &[7],
        "BB" => &[8, 9],
        "BBV" => &[10],
        "CV" => &[11],
        "SS" => &[13],
        "SSV" => &[14],
        "AV" => &[16],
        "LHA" => &[17],
        "CVB" => &[18],
        "AR" => &[19],
        "AS" => &[20],
        "CT" => &[21],
        "AO" => &[22],
        other => return Err(format!("unknown ship type {other}")),
    })
}

fn ship_types(names: &[String]) -> Result<Vec<i64>, String> {
    let mut ids = Vec::new();
    for name in names {
        ids.extend_from_slice(ship_type_ids(name)?);
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// The variants a source phase describes.
///
/// A map without a phase option has the single phase `""`, which goes to every variant the
/// map has. 7-3's two phases are its two variants. 5-6 has three phases, one per gauge, and
/// four variants: the second, where a route is opened rather than a gauge emptied, is still
/// played by the first gauge's rules.
fn variant_keys(map_id: i64, phase: &str) -> Result<&'static [&'static str], String> {
    match (map_id, phase) {
        (_, "") => Ok(&[""]),
        (73, "1") => Ok(&["pre_p_unlock"]),
        (73, "2") => Ok(&["post_p_unlock"]),
        (56, "1") => Ok(&["phase1", "phase2"]),
        (56, "2") => Ok(&["phase3"]),
        (56, "3") => Ok(&["phase4"]),
        _ => Err(format!("phase {phase} has no variant to go to")),
    }
}

/// `count op rhs` as a predicate. Counts are whole numbers, so the strict operators
/// shift the bound by one.
fn compare(
    op: &str,
    rhs: i64,
    build: impl Fn(RouteOperator, i64) -> RoutePredicate,
) -> Result<RoutePredicate, String> {
    Ok(match op {
        "==" => build(RouteOperator::Eq, rhs),
        ">=" => build(RouteOperator::Gte, rhs),
        "<=" => build(RouteOperator::Lte, rhs),
        ">" => build(RouteOperator::Gte, rhs + 1),
        "<" => build(RouteOperator::Lte, rhs - 1),
        "!=" => RoutePredicate::Not(Box::new(build(RouteOperator::Eq, rhs))),
        other => return Err(format!("unknown operator {other}")),
    })
}

fn predicate(expr: &Expr) -> Result<RoutePredicate, String> {
    Ok(match expr {
        Expr::And(parts) => {
            RoutePredicate::And(parts.iter().map(predicate).collect::<Result<_, _>>()?)
        }
        Expr::Or(parts) => {
            RoutePredicate::Or(parts.iter().map(predicate).collect::<Result<_, _>>()?)
        }
        Expr::Not(part) => RoutePredicate::Not(Box::new(predicate(part)?)),
        Expr::Cmp {
            lhs,
            op,
            rhs,
        } => {
            let terms = lhs
                .iter()
                .map(|lhs| {
                    let counter = match &lhs.term {
                        Term::ShipTypes {
                            types,
                        } => RouteCounter::ShipTypes(ship_types(types)?),
                        Term::Ships {
                            ids,
                        } => RouteCounter::Ships(ids.clone()),
                        Term::FleetSize => RouteCounter::FleetSize,
                        Term::EquipCarriers {
                            ids,
                        } => RouteCounter::EquipCarriers {
                            slotitem_ids: ids.clone(),
                        },
                        Term::SlowShips {
                            types,
                        } => RouteCounter::SlowShips {
                            ship_types: ship_types(types)?,
                        },
                    };
                    Ok(RouteCountTerm {
                        coef: lhs.coef,
                        counter,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            compare(op, *rhs, |op, value| RoutePredicate::CountSum {
                terms: terms.clone(),
                op,
                value,
            })?
        }
        Expr::Los {
            cn,
            op,
            value,
        } => {
            // Scores are floored before they are compared, which is only the same as the
            // source's comparison when the threshold is a whole number.
            if value.fract() != 0.0 {
                return Err(format!("LoS threshold {value} is not a whole number"));
            }
            if op == "==" || op == "!=" {
                return Err(format!("LoS scores are not compared with {op}"));
            }
            compare(op, *value as i64, |op, value| RoutePredicate::LoS {
                formula: None,
                coefficient: Some(*cn),
                op,
                value,
            })?
        }
        Expr::Speed {
            op,
            value,
        } => {
            let at_least = |class| RoutePredicate::Speed {
                class,
            };
            match (op.as_str(), value) {
                ("==", 1) => RoutePredicate::Not(Box::new(at_least(SpeedClass::Fast))),
                (">=", 2) => at_least(SpeedClass::Fast),
                (">=", 3) => at_least(SpeedClass::FastPlus),
                ("==" | ">=", 4) => at_least(SpeedClass::Fastest),
                _ => return Err(format!("unknown fleet speed test {op} {value}")),
            }
        }
        Expr::Visited(label) => RoutePredicate::VisitedNodeLabel {
            node_labels: vec![label.clone()],
            visited: true,
        },
        Expr::FlagshipTypes(types) => RoutePredicate::FlagshipShipType {
            ship_types: ship_types(types)?,
        },
    })
}

fn label_rules(from: &str, rules: &[Rule]) -> Result<Vec<LabelRouteRule>, String> {
    let mut out = Vec::new();
    for rule in rules {
        let predicate = match &rule.cond {
            Some(cond) => predicate(cond)?,
            None => RoutePredicate::Always,
        };
        for target in &rule.targets {
            out.push(LabelRouteRule {
                from: from.to_owned(),
                to: target.node.clone(),
                // Rates such as 0.725 pick up float noise when scaled.
                probability_pct: target.rate.map(|rate| (rate * 100_000.0).round() / 1000.0),
                // Every target of one rule carries the same predicate: that is what
                // lets the router roll among them as one group.
                predicate: predicate.clone(),
            });
        }
    }
    Ok(out)
}

fn parse_map_id(area: &str) -> Result<i64, String> {
    let (world, no) = area.split_once('-').ok_or_else(|| format!("bad map name {area}"))?;
    let parse = |part: &str| part.parse::<i64>().map_err(|_| format!("bad map name {area}"));
    Ok(parse(world)? * 10 + parse(no)?)
}

/// Turn the decoder's neutral document into the routing-rule asset.
pub fn normalize_compass_route_rules(raw: &str) -> Result<CompassRouteRulesAsset, String> {
    let document = serde_json::from_str::<Document>(raw).map_err(|err| err.to_string())?;
    let mut maps = BTreeMap::new();

    for (area, phases) in &document.maps {
        let map_id = parse_map_id(area)?;
        let mut variants = BTreeMap::new();
        for (phase, rules) in phases {
            let context = |err: String| format!("{area} phase `{phase}`: {err}");
            let keys = variant_keys(map_id, phase).map_err(context)?;
            // A map with one start has the lone rule "go to start 1", which decides nothing.
            let start = match rules.start.as_slice() {
                [only] if only.cond.is_none() => Vec::new(),
                start => label_rules("", start).map_err(context)?,
            };
            let mut variant = CompassVariantRouteRules {
                start,
                rules: Vec::new(),
            };
            for (node, node_rules) in &rules.nodes {
                variant.rules.extend(
                    label_rules(node, &node_rules.rules)
                        .map_err(|err| format!("{area} phase `{phase}` node {node}: {err}"))?,
                );
            }
            for key in keys {
                variants.insert((*key).to_owned(), variant.clone());
            }
        }
        maps.insert(map_id, variants);
    }

    Ok(CompassRouteRulesAsset {
        note: format!(
            "Routing rules for the regular maps, converted from the branch functions of the \
             compass simulator ({} at {}, {}, {}). Generated by `make route-rules-update`; \
             do not edit by hand. Node labels are the source's, with `1` and `2` naming the \
             start points.",
            document.source.repo,
            document.source.commit,
            document.source.license,
            document.source.copyright
        ),
        source: document.source,
        maps,
    })
}

/// Pin a variant's label-space rules onto its cells, replacing whatever routing rules it had.
///
/// Fails without touching the variant when a rule names a label the topology lacks or an
/// edge it does not have.
///
/// `whole` is the map's last phase when `variant` is an earlier one. The rules are written
/// for the whole map, so they must fit that; whatever an earlier phase has no cells for
/// yet is left out of it.
pub(crate) fn apply_route_rules(
    variant: &mut MapVariantDefinition,
    rules: &CompassVariantRouteRules,
    whole: Option<&MapVariantDefinition>,
) -> Result<(), Vec<String>> {
    if let Some(whole) = whole {
        apply_route_rules(&mut whole.clone(), rules, None)?;
    }
    let partial = whole.is_some();
    let label_index = variant.multi_label_index();
    // A cell the phase lacks is simply never visited.
    let visited_index =
        whole.map_or_else(|| label_index.clone(), MapVariantDefinition::multi_label_index);
    let mut starts =
        variant.start_source_cells().iter().map(|cell| cell.cell_no).collect::<Vec<_>>();
    starts.sort_unstable();
    let start_cell = |label: &str| match label {
        "1" => starts.first().copied(),
        "2" => starts.get(1).copied(),
        _ => None,
    };
    let mut errors = Vec::new();

    let mut start_rules = Vec::new();
    for rule in &rules.start {
        match start_cell(&rule.to) {
            Some(to_cell_no) => start_rules.push(RouteRule {
                from_cell_no: -1,
                to_cell_no,
                priority: 0,
                weight: rule.probability_pct.map(probability_to_weight),
                probability_pct: rule.probability_pct,
                predicate: rule.predicate.clone(),
                raw_text: String::new(),
            }),
            None if partial => {}
            None => errors.push(format!("start rule names start {}, which is missing", rule.to)),
        }
    }

    let mut routing_rules = BTreeMap::<i64, Vec<RouteRule>>::new();
    for rule in &rules.rules {
        let from_cell_nos = match start_cell(&rule.from) {
            Some(cell_no) => vec![cell_no],
            None => match label_index.get(&rule.from) {
                Some(cell_nos) => cell_nos.clone(),
                None if partial => continue,
                None => {
                    errors.push(format!(
                        "{} -> {}: no cell is labelled {}",
                        rule.from, rule.to, rule.from
                    ));
                    continue;
                }
            },
        };
        let predicate = match resolve_visited_labels(&rule.predicate, &visited_index) {
            Ok(predicate) => predicate,
            Err(label) => {
                errors.push(format!(
                    "{} -> {}: visited label {label} is missing",
                    rule.from, rule.to
                ));
                continue;
            }
        };
        let mut resolved = false;
        for from_cell_no in from_cell_nos {
            let Some(from_cell) = variant.cell(from_cell_no) else {
                continue;
            };
            for &to_cell_no in &from_cell.next_cells {
                let is_target = variant
                    .cell(to_cell_no)
                    .is_some_and(|cell| cell.node_label.as_deref() == Some(rule.to.as_str()));
                if is_target {
                    resolved = true;
                    routing_rules.entry(from_cell_no).or_default().push(RouteRule {
                        from_cell_no,
                        to_cell_no,
                        priority: 0,
                        weight: rule.probability_pct.map(probability_to_weight),
                        probability_pct: rule.probability_pct,
                        predicate: predicate.clone(),
                        raw_text: String::new(),
                    });
                }
            }
        }
        if !resolved && !partial {
            errors.push(format!("{} -> {}: the topology has no such edge", rule.from, rule.to));
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    for rules in routing_rules.values_mut().chain(std::iter::once(&mut start_rules)) {
        for (index, rule) in rules.iter_mut().enumerate() {
            rule.priority = index as i64;
        }
    }
    variant.routing_rules = routing_rules;
    variant.start_rules = start_rules;
    Ok(())
}

fn probability_to_weight(pct: f64) -> i64 {
    (pct * 100.0).round() as i64
}

/// Replace visited-label tests with the cells carrying those labels. `Err` names a label
/// no cell carries.
fn resolve_visited_labels(
    predicate: &RoutePredicate,
    label_index: &BTreeMap<String, Vec<i64>>,
) -> Result<RoutePredicate, String> {
    let all = |parts: &[RoutePredicate]| {
        parts.iter().map(|part| resolve_visited_labels(part, label_index)).collect::<Result<_, _>>()
    };
    Ok(match predicate {
        RoutePredicate::VisitedNodeLabel {
            node_labels,
            visited,
        } => {
            let mut cell_nos = Vec::new();
            for label in node_labels {
                cell_nos.extend(label_index.get(label).ok_or_else(|| label.clone())?);
            }
            cell_nos.sort_unstable();
            cell_nos.dedup();
            RoutePredicate::VisitedNode {
                cell_nos,
                visited: *visited,
            }
        }
        RoutePredicate::And(parts) => RoutePredicate::And(all(parts)?),
        RoutePredicate::Or(parts) => RoutePredicate::Or(all(parts)?),
        RoutePredicate::Not(part) => {
            RoutePredicate::Not(Box::new(resolve_visited_labels(part, label_index)?))
        }
        other => other.clone(),
    })
}

#[cfg(test)]
mod tests {
    use emukc_model::codex::map::MapCellDefinition;

    use super::*;

    fn document(maps: &str) -> String {
        format!(
            r#"{{"source":{{"repo":"o/r","commit":"abc","license":"MIT License","copyright":"Copyright (c) X"}},"maps":{maps}}}"#
        )
    }

    fn only_variant(maps: &str) -> CompassVariantRouteRules {
        let asset = normalize_compass_route_rules(&document(maps)).unwrap();
        asset.maps.into_values().next().unwrap().into_values().next().unwrap()
    }

    fn rule(cond: &str, targets: &str) -> String {
        format!(r#"{{"cond":{cond},"targets":{targets},"line":1}}"#)
    }

    fn node_map(area: &str, phase: &str, start: &str, node: &str, rules: &str) -> String {
        format!(
            r#"{{"{area}":{{"{phase}":{{"start":[{start}],"nodes":{{"{node}":{{"active":false,"rules":[{rules}]}}}}}}}}}}"#
        )
    }

    const TO_ONE: &str = r#"{"cond":null,"targets":[{"node":"1","rate":null}],"line":1}"#;

    fn cell(cell_no: i64, label: &str, next_cells: &[i64]) -> MapCellDefinition {
        MapCellDefinition {
            cell_no,
            node_label: Some(label.to_owned()),
            next_cells: next_cells.to_vec(),
            ..Default::default()
        }
    }

    #[test]
    fn count_comparison_becomes_a_count_sum_over_stype_ids() {
        let cond = r#"{"cmp":{"lhs":[{"coef":1,"term":{"kind":"ship_types","types":["BB","BBV"]}},{"coef":-1,"term":{"kind":"slow_ships","types":["BB"]}}],"op":"<","rhs":2}}"#;
        let variant = only_variant(&node_map(
            "7-4",
            "",
            TO_ONE,
            "M",
            &rule(cond, r#"[{"node":"N","rate":null}]"#),
        ));

        assert!(variant.start.is_empty(), "a lone unconditional start decides nothing");
        let [rule] = variant.rules.as_slice() else {
            panic!("expected one rule");
        };
        assert_eq!((rule.from.as_str(), rule.to.as_str()), ("M", "N"));
        let RoutePredicate::CountSum {
            terms,
            op,
            value,
        } = &rule.predicate
        else {
            panic!("expected a count sum, got {:?}", rule.predicate);
        };
        assert_eq!((*op, *value), (RouteOperator::Lte, 1), "`< 2` on a count is `<= 1`");
        assert_eq!(terms[0].counter, RouteCounter::ShipTypes(vec![8, 9, 10]));
        assert_eq!(
            terms[1],
            RouteCountTerm {
                coef: -1,
                counter: RouteCounter::SlowShips {
                    ship_types: vec![8, 9]
                }
            }
        );
    }

    #[test]
    fn los_keeps_its_coefficient_and_shifts_strict_bounds() {
        let variant = only_variant(&node_map(
            "2-5",
            "",
            TO_ONE,
            "G",
            &[
                rule(r#"{"los":{"cn":1,"op":"<","value":37}}"#, r#"[{"node":"K","rate":null}]"#),
                rule(
                    r#"{"los":{"cn":1,"op":">=","value":37}}"#,
                    r#"[{"node":"K","rate":0.5},{"node":"L","rate":0.5}]"#,
                ),
                rule("null", r#"[{"node":"L","rate":null}]"#),
            ]
            .join(","),
        ));

        let los = |op, value| RoutePredicate::LoS {
            formula: None,
            coefficient: Some(1),
            op,
            value,
        };
        let predicates = variant.rules.iter().map(|rule| format!("{:?}", rule.predicate));
        assert_eq!(
            predicates.collect::<Vec<_>>(),
            [
                los(RouteOperator::Lte, 36),
                los(RouteOperator::Gte, 37),
                los(RouteOperator::Gte, 37),
                RoutePredicate::Always
            ]
            .map(|predicate| format!("{predicate:?}"))
        );
        assert_eq!(variant.rules[1].probability_pct, Some(50.0));
        assert_eq!(variant.rules[3].probability_pct, None);
    }

    #[test]
    fn speed_visited_and_flagship_atoms_map_to_their_predicates() {
        let cond = r#"{"and":[{"speed":{"op":"==","value":1}},{"not":{"visited":"D"}},{"flagship_types":["CL"]}]}"#;
        let variant = only_variant(&node_map(
            "6-4",
            "",
            TO_ONE,
            "A",
            &rule(cond, r#"[{"node":"D","rate":null}]"#),
        ));

        assert_eq!(
            format!("{:?}", variant.rules[0].predicate),
            format!(
                "{:?}",
                RoutePredicate::And(vec![
                    RoutePredicate::Not(Box::new(RoutePredicate::Speed {
                        class: SpeedClass::Fast
                    })),
                    RoutePredicate::Not(Box::new(RoutePredicate::VisitedNodeLabel {
                        node_labels: vec!["D".into()],
                        visited: true
                    })),
                    RoutePredicate::FlagshipShipType {
                        ship_types: vec![3]
                    },
                ])
            )
        );
    }

    #[test]
    fn phases_go_to_their_variants() {
        let phase = |target: &str| {
            format!(
                r#"{{"start":[{TO_ONE}],"nodes":{{"A":{{"active":false,"rules":[{}]}}}}}}"#,
                rule("null", &format!(r#"[{{"node":"{target}","rate":null}}]"#))
            )
        };
        let maps = format!(
            r#"{{"7-3":{{"1":{},"2":{}}},"5-6":{{"1":{},"2":{},"3":{}}}}}"#,
            phase("B"),
            phase("C"),
            phase("X"),
            phase("Y"),
            phase("Z")
        );
        let asset = normalize_compass_route_rules(&document(&maps)).unwrap();

        let to = |map_id: i64, key: &str| asset.maps[&map_id][key].rules[0].to.clone();
        assert_eq!(to(73, "pre_p_unlock"), "B");
        assert_eq!(to(73, "post_p_unlock"), "C");
        assert_eq!(to(56, "phase1"), "X");
        assert_eq!(
            to(56, "phase2"),
            "X",
            "the route-opening phase plays by the first gauge's rules"
        );
        assert_eq!(to(56, "phase3"), "Y");
        assert_eq!(to(56, "phase4"), "Z");
    }

    #[test]
    fn unknown_vocabulary_is_an_error() {
        let bad_type = r#"{"cmp":{"lhs":[{"coef":1,"term":{"kind":"ship_types","types":["XX"]}}],"op":">=","rhs":1}}"#;
        let bad_los = r#"{"los":{"cn":4,"op":">=","value":45.5}}"#;
        let targets = r#"[{"node":"B","rate":null}]"#;
        for (maps, message) in [
            (node_map("1-1", "", TO_ONE, "A", &rule(bad_type, targets)), "unknown ship type XX"),
            (node_map("1-1", "", TO_ONE, "A", &rule(bad_los, targets)), "not a whole number"),
            (node_map("1-1", "2", TO_ONE, "A", &rule("null", targets)), "no variant to go to"),
        ] {
            let err = normalize_compass_route_rules(&document(&maps)).unwrap_err();
            assert!(err.contains(message), "{err}");
            assert!(err.starts_with("1-1 phase"), "{err}");
        }
    }

    /// Start 1 (cell 0) leads to A; start 2 (cell 5) leads to B. A is reached by two edges
    /// (cells 1 and 4) and leads to C (cell 2) or D (cell 3).
    fn two_start_variant() -> MapVariantDefinition {
        MapVariantDefinition {
            cells: vec![
                cell(0, "Start", &[1]),
                cell(1, "A", &[2, 3]),
                cell(2, "C", &[]),
                cell(3, "D", &[]),
                cell(4, "A", &[2, 3]),
                cell(5, "Start", &[6]),
                cell(6, "B", &[4]),
            ],
            ..Default::default()
        }
    }

    fn label_rule(from: &str, to: &str, probability_pct: Option<f64>) -> LabelRouteRule {
        LabelRouteRule {
            from: from.to_owned(),
            to: to.to_owned(),
            probability_pct,
            predicate: RoutePredicate::Always,
        }
    }

    #[test]
    fn rules_fan_out_to_every_cell_of_a_label_and_starts_stay_apart() {
        let mut variant = two_start_variant();
        let rules = CompassVariantRouteRules {
            start: vec![label_rule("", "2", None)],
            rules: vec![
                label_rule("1", "A", None),
                label_rule("2", "B", None),
                label_rule("A", "C", Some(30.0)),
                label_rule("A", "D", Some(70.0)),
            ],
        };

        apply_route_rules(&mut variant, &rules, None).unwrap();

        let edges = |from: i64| {
            variant.routing_rules[&from]
                .iter()
                .map(|rule| (rule.to_cell_no, rule.priority, rule.weight))
                .collect::<Vec<_>>()
        };
        assert_eq!(edges(0), [(1, 0, None)]);
        assert_eq!(edges(5), [(6, 0, None)]);
        assert_eq!(edges(1), [(2, 0, Some(3000)), (3, 1, Some(7000))]);
        assert_eq!(edges(4), edges(1));
        assert_eq!(variant.start_rules.iter().map(|rule| rule.to_cell_no).collect::<Vec<_>>(), [5]);
    }

    #[test]
    fn a_rule_the_topology_cannot_carry_fails_and_leaves_the_variant_alone() {
        let mut variant = two_start_variant();
        variant.routing_rules.insert(0, vec![RouteRule::default()]);
        let rules = CompassVariantRouteRules {
            start: Vec::new(),
            rules: vec![label_rule("A", "B", None), label_rule("Z", "C", None)],
        };

        let errors = apply_route_rules(&mut variant, &rules, None).unwrap_err();

        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].contains("no such edge"), "{errors:?}");
        assert!(errors[1].contains("no cell is labelled Z"), "{errors:?}");
        assert_eq!(variant.routing_rules.len(), 1, "the old rules are still there");
    }
}
