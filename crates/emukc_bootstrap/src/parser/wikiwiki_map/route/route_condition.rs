use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use emukc_model::codex::map::{RouteOperator, RoutePredicate};
use regex::Regex;

use super::super::{
    CompiledRouteClause, EnemyNodeRows, RouteClauseAst, RouteConditionLine, RouteRuleDraft,
    ShipResolver, ShipTypeResolver, WikiwikiNodeDefinition, find_header_index, is_entry_node_label,
    normalize_text, parse_node_label, parse_node_labels, parse_specific_ship_id_list,
    parse_specific_ship_list, sanitize_route_text,
};
use crate::parser::error::ParseError;

use super::route_predicate::{parse_route_predicate, unknown_predicate};

static RE_TARGET_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:で|と|は|なら|すると)(?P<target>[A-Z][A-Z0-9]?)(?:\*?\d+|\?)?$")
        .expect("valid explicit target regex")
});
static RE_PROGRESS_TARGET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<target>[A-Z][A-Z0-9]?)マス進行割合").expect("valid progress target regex")
});
static RE_TARGETED_TOKEN_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<lemma>.*?)(?:で|と|は|なら|すると)(?P<target>[A-Z][A-Z0-9]?)$")
        .expect("valid targeted token suffix regex")
});
static RE_TRAILING_ROUTE_ANNOTATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<body>.*?)[（(][^()（）]*[)）]$")
        .expect("valid trailing route annotation regex")
});
static RE_HELPER_SCOPED_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<guard>.*?)(?:(?:次の条件のいずれか|次の条件の何れか|下記のいずれかの条件|下記の何れかの条件|以下のいずれかの条件|以下の何れかの条件)を満たし|(?:以下の条件(?:をひとつ|を一つ|を)?)(?:満たし|充たし))$",
	)
	.expect("valid scoped helper header regex")
});
static RE_ROUTE_HISTORY_CONTEXT_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<label>[A-Z][A-Z0-9]?)マスを経由し、.*分岐する$")
        .expect("valid route-history context header regex")
});
static RE_FIXED_LOS_RANDOM_GATE_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^索敵スコアのランダム判定でS以外になった場合、または索敵スコア63以上の場合は下の条件に基づきルート分岐$")
		.expect("valid fixed los-random gate header regex")
});
static RE_HELPER_ELSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^上記の条件を満たさない場合(?:は|で|と)?(?P<target>[A-Z][A-Z0-9]?)?$")
        .expect("valid helper else regex")
});
static RE_ELSE_TARGET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"それ以外(?:は|で|と)?(?P<target>[A-Z][A-Z0-9]?)?$")
        .expect("valid else target regex")
});
static RE_TARGET_RANDOM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<predicate>.+?)(?:で|なら|は|と)(?P<left>[A-Z][A-Z0-9]?)(?:マス)?(?:\s*または\s*|\s*もしくは\s*|\s*又は\s*)(?P<right>[A-Z][A-Z0-9]?)(?:マス)?のランダム(?P<tail>.*)$",
	)
	.expect("valid target random regex")
});
static RE_TARGET_RANDOM_BIAS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<target>[A-Z][A-Z0-9]?)マス寄り(?:\((?P<detail>[^)]*)\))?")
        .expect("valid target random bias regex")
});
static RE_ROW_TARGET_RANDOM_BIAS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<predicate>.+?)(?:で|と|なら|は)(?P<target>[A-Z][A-Z0-9]?)(?:マス)?寄り(?:\((?P<detail>[^)]*)\))?のランダム(?:.*)$",
	)
	.expect("valid row target random bias regex")
});
static RE_ROW_TARGET_RANDOM_BIAS_SHORTHAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<predicate>.+?)(?:で|と|なら|は)(?P<target>[A-Z][A-Z0-9]?)(?:マス)?寄り(?P<detail>.*)$",
	)
	.expect("valid row target random bias shorthand regex")
});
static RE_CONDITIONAL_RANDOM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<predicate>.+?)(?:で|と|なら|は)ランダム(?:.*)$")
        .expect("valid conditional random regex")
});
static RE_BARE_BIASED_RANDOM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<target>[A-Z][A-Z0-9]?)(?:マス)?寄り[（(](?P<pct>\d+(?:\.\d+)?)[%％][）)](?:の)?ランダム$",
	)
	.expect("valid bare biased random regex")
});
static RE_HELPER_TARGET_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<guard>.*?)(?:(?:(?:次の条件のいずれか|次の条件の何れか|下記のいずれかの条件|下記の何れかの条件|以下のいずれかの条件|以下の何れかの条件)を|(?:以下の条件(?:をひとつ|を一つ|を)?))(?:満たす|充たす)と|(?:(?:次の条件のいずれか|次の条件の何れか|下記のいずれかの条件|下記の何れかの条件|以下のいずれかの条件|以下の何れかの条件)を|(?:以下の条件(?:をひとつ|を一つ|を)?))(?:満たせば|充たせば))(?P<target>[A-Z][A-Z0-9]?)$",
	)
	.expect("valid helper target header regex")
});
static RE_RESIDUAL_FLEET_HELPER_GUARD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<guard>.+?)場合[、,]?\s*他(?P<count>\d+)隻が$")
        .expect("valid residual fleet helper guard regex")
});
static RE_HELPER_RANDOM_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?:(?:次の条件のいずれか|次の条件の何れか)を|(?:以下の条件(?:をひとつ|を一つ|を)?))(?:満たす|充たす)と|(?:(?:次の条件のいずれか|次の条件の何れか)を|(?:以下の条件(?:をひとつ|を一つ|を)?))(?:満たせば|充たせば))ランダム$")
		.expect("valid helper random header regex")
});
static RE_TARGET_PROBABILITY_DISTRIBUTION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^[＿_ ]*[（(]?(?P<body>(?:[A-Z][A-Z0-9]?(?:マス)?\s*:\s*)+[A-Z][A-Z0-9]?(?:マス)?\s*=\s*\d+(?:\.\d+)?%(?:\s*:\s*\d+(?:\.\d+)?%)*)[)）]?$",
	)
	.expect("valid target probability distribution regex")
});
static RE_PROBABILITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<count>\d+)隻\s*:\s*(?P<pct>\d+(?:\.\d+)?)%").expect("valid probability regex")
});

pub fn build_nodes(
    route_rules: &[RouteRuleDraft],
    enemy_nodes: &BTreeMap<String, EnemyNodeRows>,
) -> Vec<WikiwikiNodeDefinition> {
    let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
    let mut targets = BTreeSet::new();
    let mut ordered_labels = Vec::<String>::new();

    for rule in route_rules {
        if !is_entry_node_label(&rule.from_label) && !ordered_labels.contains(&rule.from_label) {
            ordered_labels.push(rule.from_label.clone());
        }
        if !is_entry_node_label(&rule.to_label) && !ordered_labels.contains(&rule.to_label) {
            ordered_labels.push(rule.to_label.clone());
        }
        if !is_entry_node_label(&rule.from_label) {
            graph.entry(rule.from_label.clone()).or_default();
        }
        if !is_entry_node_label(&rule.to_label) {
            graph.entry(rule.to_label.clone()).or_default();
        }
        if !is_entry_node_label(&rule.from_label) && !is_entry_node_label(&rule.to_label) {
            graph.entry(rule.from_label.clone()).or_default().insert(rule.to_label.clone());
            targets.insert(rule.to_label.clone());
        }
    }

    for node_label in enemy_nodes.keys() {
        if !ordered_labels.contains(node_label) {
            ordered_labels.push(node_label.clone());
        }
        graph.entry(node_label.clone()).or_default();
    }

    let roots = ordered_labels
        .iter()
        .filter(|label| !targets.contains(*label))
        .cloned()
        .collect::<Vec<_>>();
    let mut queue = if roots.is_empty() {
        ordered_labels.clone()
    } else {
        roots
    };
    let mut seen = BTreeSet::new();
    let mut bfs = Vec::new();
    while let Some(label) = queue.first().cloned() {
        queue.remove(0);
        if !seen.insert(label.clone()) {
            continue;
        }
        bfs.push(label.clone());
        if let Some(nexts) = graph.get(&label) {
            for next in nexts {
                queue.push(next.clone());
            }
        }
    }
    for label in ordered_labels {
        if seen.insert(label.clone()) {
            bfs.push(label);
        }
    }

    let cell_numbers = bfs
        .iter()
        .enumerate()
        .map(|(idx, label)| (label.clone(), idx as i64 + 1))
        .collect::<BTreeMap<_, _>>();

    bfs.into_iter()
        .map(|label| {
            let enemy = enemy_nodes.get(&label);
            WikiwikiNodeDefinition {
                label: label.clone(),
                cell_no: cell_numbers[&label],
                is_boss: enemy.is_some_and(|node| node.is_boss),
                is_battle: enemy.is_some(),
            }
        })
        .collect()
}

pub fn parse_route_table(
    rows: &[Vec<String>],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
    warnings: &mut Vec<String>,
) -> Result<Vec<RouteRuleDraft>, ParseError> {
    let header_idx = rows
        .iter()
        .position(|row| {
            row.iter().any(|cell| cell.contains("分岐点"))
                && row.iter().any(|cell| cell.contains("移動条件"))
        })
        .ok_or_else(|| ParseError::Generic("route header row not found".to_string()))?;
    let headers = &rows[header_idx];
    let from_idx = find_header_index(headers, &["分岐点"])
        .ok_or_else(|| ParseError::Generic("route table missing `分岐点` column".to_string()))?;
    let to_idx = find_header_index(headers, &["ルート"])
        .ok_or_else(|| ParseError::Generic("route table missing `ルート` column".to_string()))?;
    let cond_idx = find_header_index(headers, &["移動条件"])
        .ok_or_else(|| ParseError::Generic("route table missing `移動条件` column".to_string()))?;

    let candidate_targets = rows
        .iter()
        .skip(header_idx + 1)
        .filter_map(|row| {
            let source_label = row.get(from_idx).and_then(|cell| parse_node_label(cell))?;
            let targets = parse_node_labels(row.get(to_idx).map_or("", |v| v.as_str()));
            if targets.is_empty() {
                return None;
            }
            let raw_text = row
                .iter()
                .skip(cond_idx)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
            Some((source_label, raw_text, targets))
        })
        .fold(
            BTreeMap::<(String, String), BTreeSet<String>>::new(),
            |mut acc, (source, raw_text, targets)| {
                let set = acc.entry((source, raw_text)).or_default();
                for target in targets {
                    set.insert(target);
                }
                acc
            },
        );

    let mut drafts = Vec::new();
    for row in rows.iter().skip(header_idx + 1) {
        let source_label = row.get(from_idx).and_then(|cell| parse_node_label(cell));
        let row_target = row.get(to_idx).and_then(|cell| parse_node_label(cell));
        let raw_text =
            row.iter().skip(cond_idx).cloned().collect::<Vec<_>>().join("\n").trim().to_string();

        let Some(source_label) = source_label else {
            continue;
        };
        let Some(row_target) = row_target else {
            continue;
        };
        let group_targets = candidate_targets
            .get(&(source_label.clone(), raw_text.clone()))
            .map(|targets| targets.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_else(|| vec![row_target.clone()]);

        if let Some(probability_target) = parse_probability_target(&raw_text)
            && probability_target != row_target
        {
            drafts.push(RouteRuleDraft {
                from_label: source_label,
                to_label: row_target,
                probability_pct: None,
                predicate: RoutePredicate::Always,
                raw_text,
                random_placeholder: true,
            });
            continue;
        }

        let parsed = parse_route_condition_text(
            &raw_text,
            &row_target,
            &group_targets,
            ship_types,
            ships,
            warnings,
        );
        if parsed.is_empty() {
            drafts.push(RouteRuleDraft {
                from_label: source_label,
                to_label: row_target,
                probability_pct: None,
                predicate: RoutePredicate::Always,
                raw_text,
                random_placeholder: false,
            });
            continue;
        }

        for parsed_clause in parsed {
            drafts.push(RouteRuleDraft {
                from_label: source_label.clone(),
                to_label: parsed_clause.target_label,
                probability_pct: parsed_clause.probability_pct,
                predicate: parsed_clause.predicate,
                raw_text: raw_text.clone(),
                random_placeholder: parsed_clause.random_placeholder,
            });
        }
    }

    let mut seen = BTreeSet::new();
    drafts.retain(|draft| seen.insert(route_rule_draft_key(draft)));
    let resolved_pairs = drafts
        .iter()
        .filter(|draft| {
            !matches!(
                draft.predicate,
                RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }
            )
        })
        .map(|draft| (draft.from_label.clone(), draft.to_label.clone()))
        .collect::<BTreeSet<_>>();
    drafts.retain(|draft| {
        !matches!(
            draft.predicate,
            RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }
        ) || !resolved_pairs.contains(&(draft.from_label.clone(), draft.to_label.clone()))
    });

    Ok(drafts)
}

/// Scan `rules` for junctions where some rules use `probability_pct` (percentage
/// encoding that maps to an integer weight via `probability_to_weight`) and
/// others have no probability set (treated as uniform weight=1).  Mixing these
/// two encoding schemes at the same junction is ambiguous.
///
/// Each detected junction appends a warning `mixed_routing_encoding_cell_<LABEL>`
/// to `warnings` (note: label strings are used here because cell numbers are
/// assigned later in `build_nodes`).  No normalisation is performed.
pub fn check_mixed_routing_encoding(rules: &[RouteRuleDraft], warnings: &mut Vec<String>) {
    // Aggregate by from_label: (has_rules_without_pct, has_rules_with_pct).
    // Skip random_placeholder rules — they will be resolved to probability or
    // Unknown predicates later and do not represent a definitive encoding choice.
    let mut seen: BTreeMap<&str, (bool, bool)> = BTreeMap::new();
    for rule in rules {
        if rule.random_placeholder {
            continue;
        }
        let entry = seen.entry(rule.from_label.as_str()).or_insert((false, false));
        if rule.probability_pct.is_some() {
            entry.1 = true;
        } else {
            entry.0 = true;
        }
    }
    for (from_label, (has_weight, has_pct)) in seen {
        if has_weight && has_pct {
            warnings.push(format!("mixed_routing_encoding_cell_{from_label}"));
        }
    }
}

pub fn postprocess_route_probabilities(rules: &mut Vec<RouteRuleDraft>) {
    let source_targets =
        rules.iter().fold(BTreeMap::<String, BTreeSet<String>>::new(), |mut acc, rule| {
            acc.entry(rule.from_label.clone()).or_default().insert(rule.to_label.clone());
            acc
        });

    let mut additions = Vec::new();
    let mut derived_sources = BTreeSet::new();
    for (from_label, targets) in source_targets {
        if targets.len() < 2 {
            continue;
        }
        let source_rules = rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.from_label == from_label)
            .collect::<Vec<_>>();

        let placeholders: Vec<_> =
            source_rules.iter().filter(|(_, rule)| rule.random_placeholder).collect();

        // Only derive a complement when there's exactly one unknown (placeholder)
        // target. Multiple unknowns → ambiguous; the fallback emits SourceUnknown.
        if placeholders.len() != 1 {
            continue;
        }
        let (_, placeholder_rule) = placeholders[0];

        // Group the probability-bearing rules by their gating predicate. Mutually
        // exclusive conditions (e.g. `FleetSize 6` vs `FleetSize 5`) are independent
        // distributions, so each yields its own complement to the placeholder target;
        // multiple targets under one condition sum into a single distribution. Summing
        // across predicates (the prior behavior) inflated the total past 100% and
        // dropped the complement entirely.
        let mut by_predicate: BTreeMap<String, (f64, RoutePredicate)> = BTreeMap::new();
        for (_, rule) in &source_rules {
            if let Some(pct) = rule.probability_pct {
                by_predicate
                    .entry(format!("{:?}", rule.predicate))
                    .or_insert_with(|| (0.0, rule.predicate.clone()))
                    .0 += pct;
            }
        }
        let mut derived_any = false;
        for (_, (sum, predicate)) in by_predicate {
            if sum >= 100.0 {
                continue;
            }
            additions.push(RouteRuleDraft {
                from_label: placeholder_rule.from_label.clone(),
                to_label: placeholder_rule.to_label.clone(),
                probability_pct: Some(100.0 - sum),
                predicate,
                raw_text: format!("{} (derived complement)", placeholder_rule.raw_text),
                random_placeholder: false,
            });
            derived_any = true;
        }
        if derived_any {
            derived_sources.insert(from_label.clone());
        }
    }

    rules.retain(|rule| !(rule.random_placeholder && derived_sources.contains(&rule.from_label)));

    for rule in rules.iter_mut() {
        if rule.random_placeholder {
            rule.predicate = unknown_predicate(rule.raw_text.clone());
        }
    }
    rules.extend(additions);
}

fn parse_route_condition_text(
    raw_text: &str,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
    warnings: &mut Vec<String>,
) -> Vec<CompiledRouteClause> {
    let text = sanitize_route_text(raw_text);
    if text.starts_with("ランダム ") || text == "ランダム" {
        let bias_text = text.strip_prefix("ランダム ").unwrap_or("");
        if !bias_text.is_empty() {
            warnings.push(format!("unsupported route condition: {bias_text}"));
        }
        return candidate_targets
            .iter()
            .map(|target| CompiledRouteClause {
                target_label: target.clone(),
                probability_pct: None,
                predicate: RoutePredicate::Always,
                random_placeholder: false,
            })
            .collect();
    }
    if let Some(parsed) = parse_case_route_condition_text(
        raw_text,
        row_target,
        candidate_targets,
        ship_types,
        ships,
        warnings,
    ) {
        return parsed;
    }
    if let Some(parsed) = parse_hardcoded_sourceunknown_block(
        raw_text,
        row_target,
        candidate_targets,
        ship_types,
        ships,
    ) {
        return parsed;
    }
    if let Some(parsed) = parse_fleet_size_probability_clauses(raw_text, row_target) {
        return parsed;
    }
    if let Some(parsed) = parse_multiline_flat_route_condition_text(
        raw_text,
        row_target,
        candidate_targets,
        ship_types,
        ships,
        warnings,
    ) {
        return parsed;
    }
    if is_ignorable_route_annotation_line(&sanitize_route_text(raw_text)) {
        return Vec::new();
    }
    let text = sanitize_route_text(raw_text);
    if text.is_empty() || matches!(text.as_str(), "それ以外" | "固定" | "能動分岐") {
        return vec![CompiledRouteClause {
            target_label: row_target.to_string(),
            probability_pct: None,
            predicate: RoutePredicate::Always,
            random_placeholder: false,
        }];
    }
    if matches!(text.as_str(), "ランダム（片寄りなし）" | "ランダム(片寄りなし)")
    {
        return vec![CompiledRouteClause {
            target_label: row_target.to_string(),
            probability_pct: None,
            predicate: RoutePredicate::Always,
            random_placeholder: false,
        }];
    }
    let probability_rules = RE_PROBABILITY
        .captures_iter(&text)
        .filter_map(|caps| {
            let count = caps.name("count")?.as_str().parse::<i64>().ok()?;
            let pct = caps.name("pct")?.as_str().parse::<f64>().ok()?;
            Some(CompiledRouteClause {
                target_label: row_target.to_string(),
                probability_pct: Some(pct),
                predicate: RoutePredicate::FleetSize {
                    op: RouteOperator::Eq,
                    value: count,
                },
                random_placeholder: false,
            })
        })
        .collect::<Vec<_>>();
    if !probability_rules.is_empty() {
        return probability_rules;
    }
    if let Some(parsed) = parse_probability_distribution_annotation_clauses(&text) {
        return parsed;
    }
    if let Some(parsed) = parse_row_target_random_bias_condition_text(
        &text,
        row_target,
        candidate_targets,
        ship_types,
        ships,
    ) {
        return parsed;
    }
    if let Some(parsed) = parse_target_random_route_condition_text(&text, ship_types, ships) {
        return parsed;
    }
    if let Some(parsed) =
        parse_conditional_random_route_condition_text(&text, candidate_targets, ship_types, ships)
    {
        return parsed;
    }
    // Bare biased random: "Dマス寄り(60%)のランダム" — unconditional with target bias.
    if let Some(caps) = RE_BARE_BIASED_RANDOM.captures(&text)
        && let (Some(target), Some(pct)) = (
            caps.name("target").map(|v| v.as_str()),
            caps.name("pct").and_then(|v| v.as_str().parse::<f64>().ok()),
        )
    {
        let complement = (100.0 - pct).max(0.0);
        return vec![
            CompiledRouteClause {
                target_label: target.to_string(),
                probability_pct: Some(pct),
                predicate: RoutePredicate::Always,
                random_placeholder: false,
            },
            CompiledRouteClause {
                target_label: row_target.to_string(),
                probability_pct: Some(complement),
                predicate: RoutePredicate::Always,
                random_placeholder: false,
            },
        ];
    }
    if text.contains("ランダム") {
        return vec![CompiledRouteClause {
            target_label: row_target.to_string(),
            probability_pct: None,
            predicate: RoutePredicate::Always,
            random_placeholder: true,
        }];
    }
    if let Some(parsed) =
        parse_bulleted_route_condition_text(&text, row_target, ship_types, ships, warnings)
    {
        return parsed
            .into_iter()
            .map(|(probability_pct, predicate, random_placeholder)| CompiledRouteClause {
                target_label: row_target.to_string(),
                probability_pct,
                predicate,
                random_placeholder,
            })
            .collect();
    }
    if let Some(parsed) = parse_inline_targeted_route_condition_text(
        &text,
        row_target,
        candidate_targets,
        ship_types,
        ships,
        warnings,
    ) {
        return parsed
            .into_iter()
            .map(|(probability_pct, predicate, random_placeholder)| CompiledRouteClause {
                target_label: row_target.to_string(),
                probability_pct,
                predicate,
                random_placeholder,
            })
            .collect();
    }
    if let Some(predicate) = parse_route_predicate(&text, ship_types, ships) {
        return vec![CompiledRouteClause {
            target_label: row_target.to_string(),
            probability_pct: None,
            predicate,
            random_placeholder: false,
        }];
    }

    let predicate = unknown_predicate(text.clone());
    if !matches!(predicate, RoutePredicate::SourceUnknown { .. }) {
        warnings.push(format!("unsupported route condition: {text}"));
    }
    vec![CompiledRouteClause {
        target_label: row_target.to_string(),
        probability_pct: None,
        predicate,
        random_placeholder: false,
    }]
}

// TODO(expiry: 2027-01): consider moving to data file
fn parse_hardcoded_sourceunknown_block(
    raw_text: &str,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<Vec<CompiledRouteClause>> {
    match sanitize_route_text(raw_text).as_str() {
        "戦艦級3隻以上でF 空母系3隻以上でF 航巡3隻以上でF それ以外はG Eへの条件は不明" =>
        {
            let mut clauses = vec![
                CompiledRouteClause {
                    target_label: "E".to_string(),
                    probability_pct: Some(20.0),
                    predicate: RoutePredicate::Always,
                    random_placeholder: false,
                },
                CompiledRouteClause {
                    target_label: "G".to_string(),
                    probability_pct: Some(80.0),
                    predicate: RoutePredicate::Always,
                    random_placeholder: false,
                },
            ];
            for predicate_text in ["戦艦級3隻以上", "空母系3隻以上", "航巡3隻以上"]
            {
                if let Some(predicate) = parse_route_predicate(predicate_text, ship_types, ships) {
                    clauses.push(CompiledRouteClause {
                        target_label: "F".to_string(),
                        probability_pct: None,
                        predicate,
                        random_placeholder: false,
                    });
                }
            }
            clauses.retain(|clause| {
                clause.target_label == row_target
                    || candidate_targets.contains(&clause.target_label)
            });
            Some(clauses)
        }
        "不明(K→Pへ進む編成は2023/02/07現在確認されていない)" => Some(
            vec![
                CompiledRouteClause {
                    target_label: "M".to_string(),
                    probability_pct: Some(95.0),
                    predicate: RoutePredicate::Always,
                    random_placeholder: false,
                },
                CompiledRouteClause {
                    target_label: "P".to_string(),
                    probability_pct: Some(5.0),
                    predicate: RoutePredicate::Always,
                    random_placeholder: false,
                },
            ]
            .into_iter()
            .filter(|clause| {
                clause.target_label == row_target
                    || candidate_targets.contains(&clause.target_label)
            })
            .collect(),
        ),
        "索敵スコア未満でI 索敵スコア以上未満で 索敵スコア以上でL" => {
            Some(
                vec![
                    CompiledRouteClause {
                        target_label: "I".to_string(),
                        probability_pct: Some(50.0),
                        predicate: RoutePredicate::Always,
                        random_placeholder: false,
                    },
                    CompiledRouteClause {
                        target_label: "L".to_string(),
                        probability_pct: Some(50.0),
                        predicate: RoutePredicate::Always,
                        random_placeholder: false,
                    },
                ]
                .into_iter()
                .filter(|clause| {
                    clause.target_label == row_target
                        || candidate_targets.contains(&clause.target_label)
                })
                .collect(),
            )
        }
        _ => None,
    }
}

fn parse_fleet_size_probability_clauses(
    raw_text: &str,
    row_target: &str,
) -> Option<Vec<CompiledRouteClause>> {
    let text = sanitize_route_text(raw_text);
    let probability_rules = RE_PROBABILITY
        .captures_iter(&text)
        .filter_map(|caps| {
            let count = caps.name("count")?.as_str().parse::<i64>().ok()?;
            let pct = caps.name("pct")?.as_str().parse::<f64>().ok()?;
            Some(CompiledRouteClause {
                target_label: row_target.to_string(),
                probability_pct: Some(pct),
                predicate: RoutePredicate::FleetSize {
                    op: RouteOperator::Eq,
                    value: count,
                },
                random_placeholder: false,
            })
        })
        .collect::<Vec<_>>();
    (!probability_rules.is_empty()).then_some(probability_rules)
}

fn parse_multiline_flat_route_condition_text(
    raw_text: &str,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
    warnings: &mut Vec<String>,
) -> Option<Vec<CompiledRouteClause>> {
    let lines = parse_route_condition_lines(raw_text);
    if lines.len() <= 1
        || lines.iter().any(|line| line.indent > 0)
        || lines.iter().any(|line| {
            strip_case_suffix(&line.text).is_some() || is_helper_target_header(&line.text)
        })
    {
        return None;
    }

    let mut clauses = Vec::new();
    let mut idx = 0;
    while let Some(line) = lines.get(idx) {
        if is_ignorable_route_annotation_line(&line.text) {
            idx += 1;
            continue;
        }
        if let Some(parsed) = parse_random_line_with_distribution_annotation(
            &line.text,
            lines
                .get(idx + 1)
                .filter(|next| next.indent >= line.indent)
                .map(|line| line.text.as_str()),
            row_target,
            candidate_targets,
            ship_types,
            ships,
        ) {
            clauses.extend(parsed);
            idx += 2;
            continue;
        }
        let parsed = parse_independent_route_condition_line(
            &line.text,
            raw_text,
            row_target,
            candidate_targets,
            ship_types,
            ships,
            warnings,
        )?;
        clauses.extend(parsed);
        idx += 1;
    }
    (!clauses.is_empty()).then_some(clauses)
}

pub fn parse_independent_route_condition_line(
    text: &str,
    context_text: &str,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
    warnings: &mut Vec<String>,
) -> Option<Vec<CompiledRouteClause>> {
    if let Some(parsed) = parse_hardcoded_sourceunknown_line(text, row_target) {
        return Some(parsed);
    }
    if let Some(parsed) = parse_probability_distribution_annotation_clauses(text) {
        return Some(parsed);
    }
    if let Some(parsed) = parse_row_target_random_bias_condition_text(
        text,
        row_target,
        candidate_targets,
        ship_types,
        ships,
    ) {
        return Some(parsed);
    }
    if let Some(parsed) = parse_row_target_random_bias_shorthand_condition_text(
        text,
        row_target,
        candidate_targets,
        ship_types,
        ships,
    ) {
        return Some(parsed);
    }
    if let Some(parsed) = parse_target_random_route_condition_text(text, ship_types, ships) {
        return Some(parsed);
    }
    if let Some(parsed) =
        parse_conditional_random_route_condition_text(text, candidate_targets, ship_types, ships)
    {
        return Some(parsed);
    }
    if let Some(explicit_target) = parse_explicit_target(text)
        && explicit_target != row_target
    {
        return Some(Vec::new());
    }
    if text.starts_with("それ以外") {
        return Some(vec![CompiledRouteClause {
            target_label: parse_else_target(text)
                .or_else(|| parse_explicit_target(text))
                .unwrap_or_else(|| row_target.to_string()),
            probability_pct: None,
            predicate: RoutePredicate::Always,
            random_placeholder: false,
        }]);
    }

    let target_label = parse_explicit_target(text).unwrap_or_else(|| row_target.to_string());
    let predicate_text = strip_explicit_target(text);
    let predicate_text = predicate_text.trim();
    let predicate = if predicate_text.is_empty() {
        RoutePredicate::Always
    } else if let Some(predicate) = parse_special_route_predicate(
        predicate_text,
        row_target,
        candidate_targets,
        context_text,
        ship_types,
        ships,
    ) {
        predicate
    } else {
        parse_route_predicate(predicate_text, ship_types, ships).unwrap_or_else(|| {
            let predicate = unknown_predicate(predicate_text.to_string());
            if !matches!(predicate, RoutePredicate::SourceUnknown { .. }) {
                warnings.push(format!("unsupported route condition: {text}"));
            }
            predicate
        })
    };
    Some(vec![CompiledRouteClause {
        target_label,
        probability_pct: None,
        predicate,
        random_placeholder: false,
    }])
}

fn parse_hardcoded_sourceunknown_line(
    text: &str,
    _row_target: &str,
) -> Option<Vec<CompiledRouteClause>> {
    match sanitize_route_text(text).as_str() {
        "Eへの条件は不明"
        | "不明(K→Pへ進む編成は2023/02/07現在確認されていない)"
        | "索敵スコア未満でI"
        | "索敵スコア以上未満で"
        | "索敵スコア以上でL" => Some(Vec::new()),
        _ => None,
    }
}

fn is_ignorable_route_annotation_line(text: &str) -> bool {
    (text.starts_with('[') && text.ends_with(']'))
        || ((text.starts_with('(') && text.ends_with(')'))
            || (text.starts_with('（') && text.ends_with('）')))
        || is_probability_modifier_note(text)
}

fn is_probability_modifier_note(text: &str) -> bool {
    (text.contains("割合") && text.contains("増やせる"))
        || (text.contains("割合") && text.contains("減らせる"))
        || text.contains("割合が0.7倍になる")
        || (text.contains("割合") && text.contains("倍になる"))
}

fn parse_random_line_with_distribution_annotation(
    text: &str,
    next_text: Option<&str>,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<Vec<CompiledRouteClause>> {
    let annotation = next_text.and_then(parse_probability_distribution_annotation_clauses)?;
    if text == "ランダム" {
        return Some(annotation);
    }
    if let Some(parsed) = parse_target_random_route_condition_text(text, ship_types, ships) {
        let explicit_targets =
            parsed.iter().map(|clause| clause.target_label.clone()).collect::<BTreeSet<_>>();
        return Some(
            annotation
                .into_iter()
                .filter(|clause| explicit_targets.contains(&clause.target_label))
                .collect(),
        );
    }
    if let Some(parsed) =
        parse_conditional_random_route_condition_text(text, candidate_targets, ship_types, ships)
    {
        let predicate = parsed.first().map(|clause| clause.predicate.clone())?;
        return Some(
            annotation
                .into_iter()
                .map(|mut clause| {
                    clause.predicate = predicate.clone();
                    clause
                })
                .collect(),
        );
    }
    if text.starts_with("それ以外")
        && (text.contains("ランダム") || text.contains("または"))
        && annotation.iter().any(|clause| clause.target_label == row_target)
    {
        return Some(annotation);
    }
    None
}

fn parse_probability_distribution_annotation_clauses(
    text: &str,
) -> Option<Vec<CompiledRouteClause>> {
    parse_probability_distribution_annotation(text).map(|entries| {
        entries
            .into_iter()
            .map(|(target_label, probability_pct)| CompiledRouteClause {
                target_label,
                probability_pct: Some(probability_pct),
                predicate: RoutePredicate::Always,
                random_placeholder: false,
            })
            .collect()
    })
}

fn parse_probability_distribution_annotation(text: &str) -> Option<Vec<(String, f64)>> {
    let text = sanitize_route_text(text);
    let body = RE_TARGET_PROBABILITY_DISTRIBUTION.captures(&text)?.name("body")?.as_str();
    let (targets, probabilities) = body.split_once('=')?;
    let targets = targets
        .split(':')
        .map(|token| normalize_text(token).trim_end_matches("マス").to_string())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let probabilities = probabilities
        .split(':')
        .map(|token| normalize_text(token).trim_end_matches('%').parse::<f64>().ok())
        .collect::<Option<Vec<_>>>()?;
    if targets.len() < 2 || targets.len() != probabilities.len() {
        return None;
    }
    Some(targets.into_iter().zip(probabilities).collect())
}

fn parse_special_route_predicate(
    text: &str,
    row_target: &str,
    candidate_targets: &[String],
    raw_text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    if let Some(predicate) = parse_los_insufficient_complement_predicate(
        text,
        row_target,
        candidate_targets,
        raw_text,
        ship_types,
        ships,
    ) {
        return Some(predicate);
    }
    None
}

fn parse_los_insufficient_complement_predicate(
    text: &str,
    row_target: &str,
    candidate_targets: &[String],
    raw_text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    if text != "索敵不足" || candidate_targets.len() != 2 {
        return None;
    }
    let other_target = candidate_targets.iter().find(|target| target.as_str() != row_target)?;
    let other_clauses = extract_targeted_clauses(raw_text)
        .into_iter()
        .filter(|(_, target)| target == other_target)
        .map(|(clause, _)| clause)
        .collect::<Vec<_>>();
    if other_clauses.is_empty() {
        return None;
    }
    let predicates = other_clauses
        .iter()
        .map(|clause| parse_route_predicate(clause, ship_types, ships))
        .collect::<Option<Vec<_>>>()?;
    if predicates.len() == 1 && predicate_is_los_only(&predicates[0]) {
        return Some(RoutePredicate::Not(Box::new(predicates.into_iter().next()?)));
    }
    let predicate = if predicates.len() == 1 {
        predicates.into_iter().next()?
    } else {
        RoutePredicate::Or(predicates)
    };
    Some(RoutePredicate::Not(Box::new(predicate)))
}

fn predicate_is_los_only(predicate: &RoutePredicate) -> bool {
    match predicate {
        RoutePredicate::LoS {
            ..
        } => true,
        RoutePredicate::And(predicates) | RoutePredicate::Or(predicates) => {
            !predicates.is_empty() && predicates.iter().all(predicate_is_los_only)
        }
        _ => false,
    }
}

pub fn parse_case_route_condition_text(
    raw_text: &str,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
    warnings: &mut Vec<String>,
) -> Option<Vec<CompiledRouteClause>> {
    let lines = parse_route_condition_lines(raw_text);
    if !lines.iter().any(|line| {
        strip_case_suffix(&line.text).is_some()
            || is_helper_target_header(&line.text)
            || is_helper_random_header(&line.text)
            || is_scoped_helper_header(&line.text)
            || is_route_history_context_header(&line.text)
            || is_fixed_los_random_gate_header(&line.text)
            || is_helper_else_line(&line.text)
    }) {
        return None;
    }

    let (clauses, _, saw_case) = parse_route_clause_list(
        &lines,
        0,
        0,
        row_target,
        &mut RouteClauseParseContext {
            candidate_targets,
            ship_types,
            ships,
            warnings,
        },
    );
    if !saw_case {
        return None;
    }

    let mut compiled = Vec::new();
    compile_route_clause_ast(&clauses, None, &mut compiled);
    Some(compiled)
}

fn parse_route_condition_lines(raw_text: &str) -> Vec<RouteConditionLine> {
    raw_text
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let indent = count_route_line_indent(line);
            let text = line.trim_start_matches(['_', '＿', '　', ' ']).trim();
            let text =
                text.strip_prefix('・').or_else(|| text.strip_prefix('･')).unwrap_or(text).trim();
            let text = sanitize_route_text(text);
            (!text.is_empty()).then_some(RouteConditionLine {
                indent,
                text,
            })
        })
        .collect()
}

struct RouteClauseParseContext<'a> {
    candidate_targets: &'a [String],
    ship_types: &'a ShipTypeResolver,
    ships: &'a ShipResolver,
    warnings: &'a mut Vec<String>,
}

fn parse_route_clause_list(
    lines: &[RouteConditionLine],
    mut idx: usize,
    indent: usize,
    row_target: &str,
    ctx: &mut RouteClauseParseContext<'_>,
) -> (Vec<RouteClauseAst>, usize, bool) {
    let mut clauses = Vec::new();
    let mut saw_case = false;

    while let Some(line) = lines.get(idx) {
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            break;
        }

        if let Some(guard_text) = strip_case_suffix(&line.text)
            && let Some(guard) = parse_route_predicate(guard_text, ctx.ship_types, ctx.ships)
        {
            let child_indent = lines
                .get(idx + 1)
                .map(|next| next.indent)
                .filter(|next_indent| *next_indent > line.indent)
                .unwrap_or(line.indent + 1);
            let (nested, next_idx, _) =
                parse_route_clause_list(lines, idx + 1, child_indent, row_target, ctx);
            clauses.push(RouteClauseAst::Case {
                guard,
                clauses: nested,
            });
            idx = next_idx;
            saw_case = true;
            continue;
        }
        if let Some((helper_guard, helper_target)) =
            parse_helper_target_header(&line.text, ctx.ship_types, ctx.ships)
        {
            let child_indent = lines
                .get(idx + 1)
                .map(|next| next.indent)
                .filter(|next_indent| *next_indent > line.indent)
                .unwrap_or(line.indent + 1);
            let (nested, next_idx, _) =
                parse_route_clause_list(lines, idx + 1, child_indent, &helper_target, ctx);
            if let Some(guard) = helper_guard {
                clauses.push(RouteClauseAst::Case {
                    guard,
                    clauses: nested,
                });
            } else {
                clauses.extend(nested);
            }
            idx = next_idx;
            saw_case = true;
            continue;
        }
        if let Some(helper_guard) =
            parse_scoped_helper_header(&line.text, ctx.ship_types, ctx.ships)
        {
            let child_indent = lines
                .get(idx + 1)
                .map(|next| next.indent)
                .filter(|next_indent| *next_indent > line.indent)
                .unwrap_or(line.indent + 1);
            let (nested, next_idx, _) =
                parse_route_clause_list(lines, idx + 1, child_indent, row_target, ctx);
            if let Some(guard) = helper_guard {
                clauses.push(RouteClauseAst::Case {
                    guard,
                    clauses: nested,
                });
            } else {
                clauses.extend(nested);
            }
            idx = next_idx;
            saw_case = true;
            continue;
        }
        if let Some(context_guard) = parse_route_history_context_header(&line.text) {
            let child_indent = lines
                .get(idx + 1)
                .map(|next| next.indent)
                .filter(|next_indent| *next_indent > line.indent)
                .unwrap_or(line.indent + 1);
            let (nested, next_idx, _) =
                parse_route_clause_list(lines, idx + 1, child_indent, row_target, ctx);
            clauses.push(RouteClauseAst::Case {
                guard: context_guard,
                clauses: nested,
            });
            idx = next_idx;
            saw_case = true;
            continue;
        }
        if let Some(fixed_guard) = parse_fixed_los_random_gate_header(&line.text) {
            let child_indent = lines
                .get(idx + 1)
                .map(|next| next.indent)
                .filter(|next_indent| *next_indent > line.indent)
                .unwrap_or(line.indent + 1);
            let (nested, next_idx, _) =
                parse_route_clause_list(lines, idx + 1, child_indent, row_target, ctx);
            clauses.push(RouteClauseAst::Case {
                guard: fixed_guard,
                clauses: nested,
            });
            idx = next_idx;
            saw_case = true;
            continue;
        }
        if is_helper_random_header(&line.text) {
            let child_indent = lines
                .get(idx + 1)
                .map(|next| next.indent)
                .filter(|next_indent| *next_indent > line.indent)
                .unwrap_or(line.indent + 1);
            let mut next_idx = idx + 1;
            while let Some(next) = lines.get(next_idx) {
                if next.indent < child_indent {
                    break;
                }
                if next.indent > child_indent {
                    break;
                }
                if let Some(predicate) =
                    parse_route_predicate(&next.text, ctx.ship_types, ctx.ships)
                {
                    for target in ctx.candidate_targets {
                        clauses.push(RouteClauseAst::Rule {
                            target_label: target.clone(),
                            probability_pct: None,
                            predicate: predicate.clone(),
                        });
                    }
                } else {
                    ctx.warnings.push(format!("unsupported route condition: {}", next.text));
                }
                next_idx += 1;
            }
            idx = next_idx;
            saw_case = true;
            continue;
        }

        if let Some(parsed) = parse_random_line_with_distribution_annotation(
            &line.text,
            lines
                .get(idx + 1)
                .filter(|next| next.indent >= line.indent)
                .map(|line| line.text.as_str()),
            row_target,
            ctx.candidate_targets,
            ctx.ship_types,
            ctx.ships,
        ) {
            for clause in parsed {
                clauses.push(RouteClauseAst::Rule {
                    target_label: clause.target_label,
                    probability_pct: clause.probability_pct,
                    predicate: clause.predicate,
                });
            }
            idx += 2;
            continue;
        }
        if line.text.starts_with("それ以外") {
            clauses.push(RouteClauseAst::Else {
                target_label: parse_else_target(&line.text)
                    .or_else(|| parse_explicit_target(&line.text))
                    .unwrap_or_else(|| row_target.to_string()),
            });
            idx += 1;
            continue;
        }
        if is_helper_else_line(&line.text) {
            clauses.push(RouteClauseAst::Else {
                target_label: parse_helper_else_target(&line.text)
                    .or_else(|| parse_explicit_target(&line.text))
                    .unwrap_or_else(|| row_target.to_string()),
            });
            idx += 1;
            continue;
        }
        if is_ignorable_route_annotation_line(&line.text) {
            idx += 1;
            continue;
        }
        if let Some(parsed) =
            parse_target_random_route_condition_text(&line.text, ctx.ship_types, ctx.ships)
        {
            for clause in parsed {
                clauses.push(RouteClauseAst::Rule {
                    target_label: clause.target_label,
                    probability_pct: clause.probability_pct,
                    predicate: clause.predicate,
                });
            }
            idx += 1;
            continue;
        }
        if let Some(parsed) = parse_row_target_random_bias_condition_text(
            &line.text,
            row_target,
            ctx.candidate_targets,
            ctx.ship_types,
            ctx.ships,
        ) {
            for clause in parsed {
                clauses.push(RouteClauseAst::Rule {
                    target_label: clause.target_label,
                    probability_pct: clause.probability_pct,
                    predicate: clause.predicate,
                });
            }
            idx += 1;
            continue;
        }
        if let Some(parsed) = parse_conditional_random_route_condition_text(
            &line.text,
            ctx.candidate_targets,
            ctx.ship_types,
            ctx.ships,
        ) {
            for clause in parsed {
                clauses.push(RouteClauseAst::Rule {
                    target_label: clause.target_label,
                    probability_pct: clause.probability_pct,
                    predicate: clause.predicate,
                });
            }
            idx += 1;
            continue;
        }

        let target_label =
            parse_explicit_target(&line.text).unwrap_or_else(|| row_target.to_string());
        let predicate_text = strip_explicit_target(&line.text);
        let predicate_text = predicate_text.trim();
        let predicate = if predicate_text.is_empty() {
            Some(RoutePredicate::Always)
        } else if let Some(predicate) = parse_special_route_predicate(
            predicate_text,
            &target_label,
            ctx.candidate_targets,
            &line.text,
            ctx.ship_types,
            ctx.ships,
        ) {
            Some(predicate)
        } else {
            parse_route_predicate(predicate_text, ctx.ship_types, ctx.ships)
        };
        if let Some(predicate) = predicate {
            clauses.push(RouteClauseAst::Rule {
                target_label,
                probability_pct: None,
                predicate,
            });
        } else {
            let predicate = unknown_predicate(line.text.clone());
            if !matches!(predicate, RoutePredicate::SourceUnknown { .. }) {
                ctx.warnings.push(format!("unsupported route condition: {}", line.text));
            }
            clauses.push(RouteClauseAst::Rule {
                target_label,
                probability_pct: None,
                predicate,
            });
        }
        idx += 1;
    }

    (clauses, idx, saw_case)
}

fn strip_case_suffix(text: &str) -> Option<&str> {
    ["の場合", "場合", "のとき", "とき", "の時", "時"]
        .into_iter()
        .find_map(|suffix| text.strip_suffix(suffix))
        .map(str::trim)
}

fn count_route_line_indent(raw: &str) -> usize {
    raw.chars().take_while(|c| matches!(c, '_' | '＿' | '　')).count()
}

fn is_helper_target_header(text: &str) -> bool {
    RE_HELPER_TARGET_HEADER.is_match(text)
}

fn is_helper_random_header(text: &str) -> bool {
    RE_HELPER_RANDOM_HEADER.is_match(text)
}

fn is_scoped_helper_header(text: &str) -> bool {
    RE_HELPER_SCOPED_HEADER.is_match(text)
}

fn is_route_history_context_header(text: &str) -> bool {
    RE_ROUTE_HISTORY_CONTEXT_HEADER.is_match(text)
}

fn is_fixed_los_random_gate_header(text: &str) -> bool {
    RE_FIXED_LOS_RANDOM_GATE_HEADER.is_match(text)
}

fn is_helper_else_line(text: &str) -> bool {
    RE_HELPER_ELSE.is_match(text)
}

fn parse_helper_target_header(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<(Option<RoutePredicate>, String)> {
    let caps = RE_HELPER_TARGET_HEADER.captures(text)?;
    let target = caps.name("target")?.as_str().to_string();
    let mut guard_text = sanitize_route_text(caps.name("guard")?.as_str());
    let mut residual_fleet_size = None::<i64>;
    for suffix in [
        format!("またはランダムで{target}の際に"),
        format!("またはランダムで{target}マスの際に"),
        "の際に".to_string(),
    ] {
        if let Some(trimmed) = guard_text.strip_suffix(&suffix) {
            guard_text = sanitize_route_text(trimmed);
            break;
        }
    }
    if let Some((trimmed_guard, fleet_size)) = parse_residual_fleet_helper_guard(&guard_text, ships)
    {
        guard_text = trimmed_guard;
        residual_fleet_size = Some(fleet_size);
    }
    if guard_text.is_empty() {
        return Some((None, target));
    }
    guard_text = trim_trailing_route_conjunction(&guard_text);
    let mut guard = parse_route_predicate(&guard_text, ship_types, ships)?;
    if let Some(fleet_size) = residual_fleet_size {
        guard = combine_route_predicates(
            Some(guard),
            RoutePredicate::FleetSize {
                op: RouteOperator::Eq,
                value: fleet_size,
            },
        );
    }
    Some((Some(guard), target))
}

fn parse_residual_fleet_helper_guard(text: &str, ships: &ShipResolver) -> Option<(String, i64)> {
    let caps = RE_RESIDUAL_FLEET_HELPER_GUARD.captures(text)?;
    let guard_text = sanitize_route_text(caps.name("guard")?.as_str());
    let remaining = caps.name("count")?.as_str().parse::<i64>().ok()?;
    let guard_count = estimate_guard_ship_count(&guard_text, ships)?;
    Some((guard_text, remaining + guard_count))
}

fn estimate_guard_ship_count(text: &str, ships: &ShipResolver) -> Option<i64> {
    let text = sanitize_route_text(text);
    for suffix in ["を含む", "含む", "を含み", "含み"] {
        let Some(token) = text.strip_suffix(suffix).map(str::trim) else {
            continue;
        };
        if let Some(base) = token.strip_suffix("の両方").map(str::trim)
            && let Some(ship_ids) = parse_specific_ship_id_list(base, ships)
        {
            return Some(ship_ids.len() as i64);
        }
        if let Some(base) = token
            .strip_suffix("のいずれも")
            .or_else(|| token.strip_suffix("のどちらも"))
            .map(str::trim)
            && let Some(ship_ids) = parse_specific_ship_id_list(base, ships)
        {
            return Some(ship_ids.len() as i64);
        }
        if let Some(ship_ids) = parse_specific_ship_id_list(token, ships) {
            return Some(ship_ids.len() as i64);
        }
        if parse_specific_ship_list(token, ships).is_some() {
            return Some(1);
        }
    }
    None
}

fn parse_scoped_helper_header(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<Option<RoutePredicate>> {
    let caps = RE_HELPER_SCOPED_HEADER.captures(text)?;
    let guard_text =
        trim_trailing_route_conjunction(&sanitize_route_text(caps.name("guard")?.as_str()));
    if guard_text.is_empty() {
        return Some(None);
    }
    parse_route_predicate(&guard_text, ship_types, ships).map(Some)
}

fn parse_route_history_context_header(text: &str) -> Option<RoutePredicate> {
    let caps = RE_ROUTE_HISTORY_CONTEXT_HEADER.captures(text)?;
    let label = caps.name("label")?.as_str().to_string();
    Some(RoutePredicate::VisitedNodeLabel {
        node_labels: vec![label],
        visited: true,
    })
}

fn parse_fixed_los_random_gate_header(text: &str) -> Option<RoutePredicate> {
    RE_FIXED_LOS_RANDOM_GATE_HEADER.is_match(text).then_some(RoutePredicate::LoS {
        formula: None,
        op: RouteOperator::Gte,
        value: 58,
    })
}

fn trim_trailing_route_conjunction(text: &str) -> String {
    let trimmed = text.trim();
    if let Some(base) = trimmed.strip_suffix("かつ").or_else(|| trimmed.strip_suffix("且つ")) {
        sanitize_route_text(base)
    } else {
        trimmed.to_string()
    }
}

fn parse_helper_else_target(text: &str) -> Option<String> {
    RE_HELPER_ELSE
        .captures(&sanitize_route_text(text))
        .and_then(|caps| caps.name("target"))
        .map(|value| value.as_str().to_string())
}

fn compile_route_clause_ast(
    clauses: &[RouteClauseAst],
    inherited_guard: Option<RoutePredicate>,
    compiled: &mut Vec<CompiledRouteClause>,
) {
    let mut sibling_predicates: Vec<RoutePredicate> = Vec::new();

    for clause in clauses {
        match clause {
            RouteClauseAst::Rule {
                target_label,
                probability_pct,
                predicate,
            } => {
                sibling_predicates.push(predicate.clone());
                compiled.push(CompiledRouteClause {
                    target_label: target_label.clone(),
                    probability_pct: *probability_pct,
                    predicate: combine_route_predicates(inherited_guard.clone(), predicate.clone()),
                    random_placeholder: false,
                });
            }
            RouteClauseAst::Case {
                guard,
                clauses,
            } => {
                sibling_predicates.push(guard.clone());
                compile_route_clause_ast(
                    clauses,
                    Some(combine_route_predicates(inherited_guard.clone(), guard.clone())),
                    compiled,
                );
            }
            RouteClauseAst::Else {
                target_label,
            } => {
                let negated = if sibling_predicates.is_empty()
                    || sibling_predicates.iter().any(predicate_contains_unknown)
                {
                    RoutePredicate::Always
                } else {
                    let union = match sibling_predicates.len() {
                        1 => sibling_predicates[0].clone(),
                        _ => RoutePredicate::Or(sibling_predicates.clone()),
                    };
                    RoutePredicate::Not(Box::new(union))
                };
                compiled.push(CompiledRouteClause {
                    target_label: target_label.clone(),
                    probability_pct: None,
                    predicate: combine_route_predicates(inherited_guard.clone(), negated),
                    random_placeholder: false,
                });
            }
        }
    }
}

fn predicate_contains_unknown(predicate: &RoutePredicate) -> bool {
    match predicate {
        RoutePredicate::Unknown {
            ..
        }
        | RoutePredicate::SourceUnknown {
            ..
        } => true,
        RoutePredicate::And(ps) | RoutePredicate::Or(ps) => {
            ps.iter().any(predicate_contains_unknown)
        }
        RoutePredicate::Not(p) => predicate_contains_unknown(p),
        _ => false,
    }
}

fn combine_route_predicates(left: Option<RoutePredicate>, right: RoutePredicate) -> RoutePredicate {
    match left {
        None => right,
        Some(RoutePredicate::And(mut predicates)) => {
            predicates.push(right);
            RoutePredicate::And(predicates)
        }
        Some(left) => RoutePredicate::And(vec![left, right]),
    }
}

fn parse_bulleted_route_condition_text(
    text: &str,
    row_target: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
    _warnings: &mut Vec<String>,
) -> Option<Vec<(Option<f64>, RoutePredicate, bool)>> {
    if !text.contains('・') || !text.contains("何れか") {
        return None;
    }

    let items = text
        .split('・')
        .map(sanitize_route_text)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if items.is_empty() {
        return None;
    }

    let mut explicit_targets = BTreeSet::new();
    let mut active_target = None::<String>;
    let mut active_terms = Vec::<String>::new();
    let mut parsed = Vec::new();
    let mut else_target = None::<Option<String>>;
    let mut saw_group = false;

    type FlushParsed = Vec<(Option<f64>, RoutePredicate, bool)>;
    let flush_group = |target: &Option<String>,
                       terms: &mut Vec<String>,
                       parsed: &mut FlushParsed|
     -> Option<()> {
        let target = target.as_ref()?;
        if target != row_target || terms.is_empty() {
            terms.clear();
            return Some(());
        }
        let predicates = terms
            .iter()
            .map(|term| parse_route_predicate(term, ship_types, ships))
            .collect::<Option<Vec<_>>>()?;
        terms.clear();
        let predicate = if predicates.len() == 1 {
            predicates.into_iter().next()?
        } else {
            RoutePredicate::Or(predicates)
        };
        parsed.push((None, predicate, false));
        Some(())
    };

    for item in items {
        if item.contains("何れか")
            && let Some(target) = parse_clause_target(&item)
        {
            saw_group = true;
            explicit_targets.insert(target.clone());
            flush_group(&active_target, &mut active_terms, &mut parsed)?;
            active_target = Some(target);
            continue;
        }
        if item.starts_with("それ以外") {
            saw_group = true;
            flush_group(&active_target, &mut active_terms, &mut parsed)?;
            else_target = Some(parse_else_target(&item));
            active_target = None;
            continue;
        }
        if active_target.is_some() {
            active_terms.push(item);
        }
    }
    flush_group(&active_target, &mut active_terms, &mut parsed)?;

    if !saw_group {
        return None;
    }

    match else_target {
        Some(Some(target)) if target == row_target => {
            parsed.push((None, RoutePredicate::Always, false));
        }
        Some(None) if !explicit_targets.contains(row_target) => {
            parsed.push((None, RoutePredicate::Always, false));
        }
        _ => {}
    }

    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

pub fn parse_inline_targeted_route_condition_text(
    text: &str,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
    warnings: &mut Vec<String>,
) -> Option<Vec<(Option<f64>, RoutePredicate, bool)>> {
    let clauses = extract_targeted_clauses(text);
    if clauses.is_empty() {
        return None;
    }

    let explicit_targets =
        clauses.iter().map(|(_, target)| target.clone()).collect::<BTreeSet<_>>();
    let mut parsed = clauses
        .into_iter()
        .filter_map(|(clause, target)| (target == row_target).then_some(clause))
        .map(|clause| {
            parse_special_route_predicate(
                &clause,
                row_target,
                candidate_targets,
                text,
                ship_types,
                ships,
            )
            .or_else(|| parse_route_predicate(&clause, ship_types, ships))
            .map_or_else(
                || {
                    warnings.push(format!("unsupported route condition: {text}"));
                    (None, unknown_predicate(clause), false)
                },
                |predicate| (None, predicate, false),
            )
        })
        .collect::<Vec<_>>();

    match parse_else_target(text) {
        Some(target) if target == row_target => parsed.push((None, RoutePredicate::Always, false)),
        None if text.contains("それ以外") && !explicit_targets.contains(row_target) => {
            parsed.push((None, RoutePredicate::Always, false));
        }
        _ => {}
    }

    if parsed.is_empty() {
        None
    } else {
        Some(parsed)
    }
}

pub fn parse_target_random_route_condition_text(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<Vec<CompiledRouteClause>> {
    let caps = RE_TARGET_RANDOM.captures(text)?;
    let predicate_text = sanitize_route_text(caps.name("predicate")?.as_str());
    let predicate = parse_route_predicate(&predicate_text, ship_types, ships)?;
    let left = caps.name("left")?.as_str().to_string();
    let right = caps.name("right")?.as_str().to_string();
    let tail = caps.name("tail").map(|value| normalize_text(value.as_str())).unwrap_or_default();
    let probabilities = parse_target_random_probabilities(&left, &right, &tail);

    Some(vec![
        CompiledRouteClause {
            target_label: left,
            probability_pct: probabilities.0,
            predicate: predicate.clone(),
            random_placeholder: false,
        },
        CompiledRouteClause {
            target_label: right,
            probability_pct: probabilities.1,
            predicate,
            random_placeholder: false,
        },
    ])
}

fn parse_target_random_probabilities(
    left: &str,
    right: &str,
    tail: &str,
) -> (Option<f64>, Option<f64>) {
    let Some(caps) = RE_TARGET_RANDOM_BIAS.captures(tail) else {
        return (None, None);
    };
    let Some(target) = caps.name("target").map(|value| value.as_str()) else {
        return (None, None);
    };
    let pct = parse_bias_probability(caps.name("detail").map(|value| value.as_str()));
    if target == left {
        (pct, pct.map(|value| (100.0 - value).max(0.0)))
    } else if target == right {
        (pct.map(|value| (100.0 - value).max(0.0)), pct)
    } else {
        (None, None)
    }
}

pub fn parse_row_target_random_bias_condition_text(
    text: &str,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<Vec<CompiledRouteClause>> {
    let caps = RE_ROW_TARGET_RANDOM_BIAS.captures(text)?;
    let predicate_text = sanitize_route_text(caps.name("predicate")?.as_str());
    let predicate = if predicate_text == "それ以外" {
        RoutePredicate::Always
    } else {
        parse_route_predicate(&predicate_text, ship_types, ships)?
    };
    let target = caps.name("target")?.as_str();
    let pct = parse_bias_probability(caps.name("detail").map(|value| value.as_str()));
    let targets = if candidate_targets.is_empty() {
        vec![row_target.to_string()]
    } else {
        candidate_targets.to_vec()
    };
    let other_targets = targets
        .iter()
        .filter(|candidate| candidate.as_str() != target)
        .cloned()
        .collect::<Vec<_>>();

    if other_targets.len() == 1 {
        let other_target = other_targets[0].clone();
        let complement = pct.map(|value| (100.0 - value).max(0.0));
        return Some(vec![
            CompiledRouteClause {
                target_label: target.to_string(),
                probability_pct: pct,
                predicate: predicate.clone(),
                random_placeholder: false,
            },
            CompiledRouteClause {
                target_label: other_target,
                probability_pct: complement,
                predicate,
                random_placeholder: false,
            },
        ]);
    }

    if target == row_target {
        Some(vec![CompiledRouteClause {
            target_label: row_target.to_string(),
            probability_pct: pct,
            predicate,
            random_placeholder: false,
        }])
    } else {
        Some(vec![CompiledRouteClause {
            target_label: row_target.to_string(),
            probability_pct: None,
            predicate,
            random_placeholder: true,
        }])
    }
}

pub fn parse_row_target_random_bias_shorthand_condition_text(
    text: &str,
    row_target: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<Vec<CompiledRouteClause>> {
    let caps = RE_ROW_TARGET_RANDOM_BIAS_SHORTHAND.captures(text)?;
    let predicate_text = sanitize_route_text(caps.name("predicate")?.as_str());
    let predicate = if predicate_text == "それ以外" {
        RoutePredicate::Always
    } else {
        parse_route_predicate(&predicate_text, ship_types, ships)?
    };
    let target = caps.name("target")?.as_str();
    let detail = caps.name("detail")?.as_str().trim();
    if detail.contains("のランダム") || detail.is_empty() {
        return None;
    }
    let pct = parse_bias_probability(Some(detail));
    let targets = if candidate_targets.is_empty() {
        vec![row_target.to_string()]
    } else {
        candidate_targets.to_vec()
    };
    let other_targets = targets
        .iter()
        .filter(|candidate| candidate.as_str() != target)
        .cloned()
        .collect::<Vec<_>>();
    if other_targets.len() != 1 {
        return None;
    }
    let other_target = other_targets[0].clone();
    let complement = pct.map(|value| (100.0 - value).max(0.0));
    Some(vec![
        CompiledRouteClause {
            target_label: target.to_string(),
            probability_pct: pct,
            predicate: predicate.clone(),
            random_placeholder: false,
        },
        CompiledRouteClause {
            target_label: other_target,
            probability_pct: complement,
            predicate,
            random_placeholder: false,
        },
    ])
}

fn parse_bias_probability(detail: Option<&str>) -> Option<f64> {
    let detail = detail.map(normalize_text)?;
    if detail.is_empty() || detail.contains('~') || detail.contains("前後") {
        return None;
    }
    let value = detail.trim_end_matches(['%', '％', '?', '？']).trim().parse::<f64>().ok()?;
    Some(value)
}

pub fn parse_conditional_random_route_condition_text(
    text: &str,
    candidate_targets: &[String],
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<Vec<CompiledRouteClause>> {
    let caps = RE_CONDITIONAL_RANDOM.captures(text)?;
    let predicate_text = sanitize_route_text(caps.name("predicate")?.as_str());
    let predicate = if predicate_text == "それ以外" {
        RoutePredicate::Always
    } else {
        parse_route_predicate(&predicate_text, ship_types, ships)?
    };
    let targets = if candidate_targets.is_empty() {
        return None;
    } else {
        candidate_targets.to_vec()
    };
    Some(
        targets
            .into_iter()
            .map(|target_label| CompiledRouteClause {
                target_label,
                probability_pct: None,
                predicate: predicate.clone(),
                random_placeholder: false,
            })
            .collect(),
    )
}

fn extract_targeted_clauses(text: &str) -> Vec<(String, String)> {
    let mut clauses = Vec::new();
    let mut tokens = Vec::<String>::new();
    for token in sanitize_route_text(text).split(' ') {
        if token.is_empty() {
            continue;
        }
        tokens.push(token.to_string());
        if let Some((lemma, target)) = split_targeted_token(token) {
            let mut terms = tokens[..tokens.len().saturating_sub(1)].to_vec();
            if !lemma.is_empty() {
                terms.push(lemma);
            }
            clauses.push((sanitize_route_text(&terms.join(" ")), target));
            tokens.clear();
        }
    }
    clauses
}

fn split_targeted_token(token: &str) -> Option<(String, String)> {
    let caps = RE_TARGETED_TOKEN_SUFFIX.captures(token)?;
    let lemma = sanitize_route_text(caps.name("lemma")?.as_str());
    let target = caps.name("target")?.as_str().to_string();
    Some((lemma, target))
}

fn route_rule_draft_key(draft: &RouteRuleDraft) -> String {
    let predicate = serde_json::to_string(&draft.predicate)
        .unwrap_or_else(|_| format!("{:?}", draft.predicate));
    format!(
        "{}|{}|{:?}|{}|{}|{}",
        draft.from_label,
        draft.to_label,
        draft.probability_pct,
        predicate,
        draft.raw_text,
        draft.random_placeholder
    )
}

fn parse_explicit_target(text: &str) -> Option<String> {
    let normalized = strip_trailing_route_annotation(text);
    RE_TARGET_SUFFIX
        .captures(&normalized)
        .and_then(|caps| caps.name("target"))
        .map(|value| value.as_str().to_string())
}

fn parse_probability_target(text: &str) -> Option<String> {
    RE_PROGRESS_TARGET
        .captures(text)
        .and_then(|caps| caps.name("target"))
        .map(|value| value.as_str().to_string())
}

fn parse_clause_target(text: &str) -> Option<String> {
    extract_targeted_clauses(text).last().map(|(_, target)| target.clone())
}

fn parse_else_target(text: &str) -> Option<String> {
    RE_ELSE_TARGET
        .captures(&sanitize_route_text(text))
        .and_then(|caps| caps.name("target"))
        .map(|value| value.as_str().to_string())
}

fn strip_explicit_target(text: &str) -> String {
    let normalized = strip_trailing_route_annotation(text);
    RE_TARGET_SUFFIX.replace(&normalized, "").trim().to_string()
}

fn strip_trailing_route_annotation(text: &str) -> String {
    let mut text = sanitize_route_text(text);
    while let Some(caps) = RE_TRAILING_ROUTE_ANNOTATION.captures(&text) {
        let body = caps.name("body").map(|value| value.as_str()).unwrap_or_default().trim();
        if body.is_empty() {
            break;
        }
        if ["かつ", "且つ", "または", "もしくは", "又は"]
            .iter()
            .any(|suffix| body.ends_with(suffix))
        {
            break;
        }
        text = body.to_string();
    }
    text
}
