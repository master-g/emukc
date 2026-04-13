use std::{
    collections::{BTreeMap, BTreeSet},
    sync::LazyLock,
};

use emukc_model::codex::map::{
    EnemyComposition, FleetSizeWeight, RouteOperator, RoutePredicate, SpeedClass,
};
use regex::Regex;
use scraper::Html;

use super::{
    CompiledRouteClause, EnemyNodeRows, RouteClauseAst, RouteConditionLine, RouteRuleDraft,
    RouteTableSection, SELECTOR_FOLD_CONTAINER, SELECTOR_TABLE, ShipResolver, ShipTypeResolver,
    WikiwikiNodeDefinition, direct_child_tables, direct_child_with_class, find_header_index,
    is_entry_node_label, normalize_text, parse_named_pair_contains_predicate, parse_node_label,
    parse_ship_selector_count_clause, parse_ship_type_count_clause, parse_specific_ship_id_list,
    parse_specific_ship_list, predicate_for_contains_selector, predicate_for_only_selector,
    resolve_route_selector, sanitize_route_text, table_to_grid,
};
use crate::parser::error::ParseError;

static RE_PROBABILITY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<count>\d+)隻\s*:\s*(?P<pct>\d+(?:\.\d+)?)%").expect("valid probability regex")
});
static RE_FLEET_SIZE_EQ: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?P<count>\d+)隻(?:の)?編成").expect("valid fleet size regex"));
static RE_FLEET_SIZE_LTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<count>\d+)隻以下(?:の?編成)?").expect("valid fleet size lte regex")
});
static RE_FLEET_SIZE_GTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<count>\d+)隻以上(?:の?編成)?").expect("valid fleet size gte regex")
});
static RE_DRUM_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"ドラム缶(?:\(輸送用\))?を?(?P<count>\d+)個以上")
        .expect("valid drum canister regex")
});
static RE_LOS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"索敵値(?P<formula>[^で]+)?で(?P<value>\d+)以上").expect("valid los regex")
});
static RE_LOS_RANGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"索敵(?:スコア|値)(?:が)?(?P<min>\d+)以上(?P<max>\d+)未満")
        .expect("valid los range regex")
});
static RE_LOS_RANGE_BARE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<max>\d+)未満(?P<min>\d+)以上(?:のとき|の場合)?$")
        .expect("valid bare los range regex")
});
static RE_LOS_RANGE_ALT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<min>\d+)以上(?P<max>\d+)未満(?:のとき|の場合)?$")
        .expect("valid alt bare los range regex")
});
static RE_LOS_RANGE_LTE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"索敵(?:スコア|値)(?:が)?(?P<min>\d+)以上(?P<max>\d+)以下")
        .expect("valid los inclusive range regex")
});
static RE_LOS_RANGE_LTE_BARE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<min>\d+)以上(?P<max>\d+)以下(?:のとき|の場合)?$")
        .expect("valid bare inclusive los range regex")
});
static RE_LOS_GTE_SIMPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"索敵(?:スコア|値)(?:が)?(?P<value>\d+)以上").expect("valid los gte regex")
});
static RE_LOS_LTE_SIMPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"索敵(?:スコア|値)(?:が)?(?P<value>\d+)以下").expect("valid los lte regex")
});
static RE_LOS_LT_SIMPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"索敵(?:スコア|値)(?:が)?(?P<value>\d+)未満").expect("valid los lt regex")
});
static RE_LOS_GTE_BARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<value>\d+)以上$").expect("valid bare los gte regex"));
static RE_LOS_LTE_BARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<value>\d+)以下$").expect("valid bare los lte regex"));
static RE_LOS_LT_BARE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<value>\d+)未満$").expect("valid bare los lt regex"));
static RE_TARGET_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:で|と|は|なら|すると)(?P<target>[A-Z][A-Z0-9]?)(?:\*?\d+|\?)?$")
        .expect("valid explicit target regex")
});
static RE_PROGRESS_TARGET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?P<target>[A-Z][A-Z0-9]?)マス進行割合").expect("valid progress target regex")
});
static RE_SHIP_TYPE_CONTAINS_COUNT_CLAUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<name>.+?)(?P<count>\d+)隻(?P<op>以上|以下|ちょうど|過不足なく)?を含(?:む|み|まない)$",
	)
	.expect("valid ship type contains count clause regex")
});
static RE_CONTAINS_COUNT_OP_BEFORE_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<name>.+?)[\s（(]*(?P<op>過不足なく|ちょうど)[\s）)]*(?P<count>\d+(?:隻)?)を含(?P<suffix>む|み|まない)$",
	)
	.expect("valid contains count-op-before-count regex")
});
static RE_TARGETED_TOKEN_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<lemma>.*?)(?:で|と|は|なら|すると)(?P<target>[A-Z][A-Z0-9]?)$")
        .expect("valid targeted token suffix regex")
});
static RE_TRAILING_ROUTE_ANNOTATION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<body>.*?)[（(][^()（）]*[)）]$")
        .expect("valid trailing route annotation regex")
});
static RE_EQUIPMENT_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<name>.+?)(?:を装備した艦|搭載艦の隻数)が(?P<count>\d+)隻?(?P<op>以上|以下)?$")
        .expect("valid equipment count regex")
});
static RE_FLAGSHIP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<name>.+?)旗艦$").expect("valid flagship regex"));
static RE_VISITED_NODE_NEGATIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<label>[A-Z][A-Z0-9]?)マス未経由$").expect("valid visited negative regex")
});
static RE_VISITED_NODE_POSITIVE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<label>[A-Z][A-Z0-9]?)マスを経由(?:済み)?$")
        .expect("valid visited positive regex")
});
static RE_SPEED_QUALIFIED_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<speed>(?:低速|\(低速\)|（低速）|高速\+|高速＋|高速|最速))(?:\s*)?(?P<name>.+?)(?P<count>\d+)(?P<op>以上|以下|ちょうど|過不足なく)?$",
	)
	.expect("valid speed-qualified selector count regex")
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

pub(super) fn find_route_table_sections(document: &Html) -> Vec<RouteTableSection> {
    let mut sections = document
        .select(&SELECTOR_FOLD_CONTAINER)
        .flat_map(|container| {
            let summary = direct_child_with_class(&container, "fold-summary")
                .map(|summary| normalize_text(&summary.text().collect::<Vec<_>>().join(" ")))
                .unwrap_or_default();
            let Some(content) = direct_child_with_class(&container, "fold-content") else {
                return Vec::new().into_iter();
            };

            direct_child_tables(&content)
                .into_iter()
                .filter_map(move |table| {
                    let rows = table_to_grid(&table);
                    is_route_table_rows(&rows).then_some(RouteTableSection {
                        summary: summary.clone(),
                        rows,
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect::<Vec<_>>();
    if sections.is_empty()
        && let Some(rows) =
            super::find_table_by_headers(document, &["分岐点", "ルート", "移動条件"])
    {
        sections.push(RouteTableSection {
            summary: String::new(),
            rows,
        });
    }
    sections
}

fn is_route_table_rows(rows: &[Vec<String>]) -> bool {
    let haystack =
        rows.iter().take(3).flat_map(|row| row.iter()).cloned().collect::<Vec<_>>().join(" ");
    ["分岐点", "ルート", "移動条件"].iter().all(|header| haystack.contains(header))
}

pub(super) fn parse_gauge_defeat_counts(document: &Html) -> Vec<i64> {
    document
        .select(&SELECTOR_TABLE)
        .find_map(|table| {
            let rows = table_to_grid(&table);
            let counts =
                rows.iter().filter_map(|row| parse_gauge_defeat_count_row(row)).collect::<Vec<_>>();
            (counts.len() >= 2).then_some(counts)
        })
        .unwrap_or_default()
}

fn parse_gauge_defeat_count_row(row: &[String]) -> Option<i64> {
    static RE_DEFEAT_COUNT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?P<count>\d+)回撃沈").expect("valid defeat count regex"));

    let header = row.first()?;
    if !header.contains("ゲージ") {
        return None;
    }
    row.iter().find_map(|cell| {
        RE_DEFEAT_COUNT
            .captures(cell)
            .and_then(|caps| caps.name("count"))
            .and_then(|count| count.as_str().parse::<i64>().ok())
    })
}

pub(super) fn route_section_variant_key(summary: &str, idx: usize, total: usize) -> String {
    if total <= 1 {
        return String::new();
    }
    let summary_compact = summary.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    if summary_compact.contains("Pマス出現前") {
        "pre_p_unlock".to_string()
    } else if summary_compact.contains("Pマス出現後") {
        "post_p_unlock".to_string()
    } else if summary_compact.contains("第一ゲージ") || summary_compact.contains("ゲージ1")
    {
        "gauge_1".to_string()
    } else if summary_compact.contains("第二ゲージ") || summary_compact.contains("ゲージ2")
    {
        "gauge_2".to_string()
    } else {
        format!("variant_{}", idx + 1)
    }
}

pub(super) fn filter_enemy_nodes_for_route_rules(
    route_rules: &[RouteRuleDraft],
    enemy_nodes: &BTreeMap<String, EnemyNodeRows>,
) -> BTreeMap<String, EnemyNodeRows> {
    let labels = route_rules
        .iter()
        .flat_map(|rule| [rule.from_label.clone(), rule.to_label.clone()])
        .collect::<BTreeSet<_>>();
    enemy_nodes
        .iter()
        .filter(|(label, _)| labels.contains(*label))
        .map(|(label, node)| (label.clone(), node.clone()))
        .collect()
}

pub(super) fn collect_formations(compositions: &[EnemyComposition]) -> Vec<i64> {
    let mut formations = compositions
        .iter()
        .filter_map(|composition| composition.formation)
        .collect::<BTreeSet<_>>();
    if formations.is_empty() {
        formations.insert(1);
    }
    formations.into_iter().collect()
}

pub(super) fn unknown_predicate(raw_text: String) -> RoutePredicate {
    let sanitized = sanitize_route_text(&raw_text);
    if raw_text.contains("不明") || is_incomplete_los_source_text(&sanitized) {
        RoutePredicate::SourceUnknown {
            raw_text,
        }
    } else {
        RoutePredicate::Unknown {
            raw_text,
        }
    }
}

fn is_incomplete_los_source_text(text: &str) -> bool {
    text.contains("索敵")
        && (text.contains("未満") || text.contains("以上") || text.contains("以下"))
        && !text.chars().any(|ch| ch.is_ascii_digit())
}

pub(super) fn compact_route_raw_text(predicate: &RoutePredicate, raw_text: String) -> String {
    if matches!(predicate, RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }) {
        raw_text
    } else {
        String::new()
    }
}

pub(super) fn build_nodes(
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
            let next_cells = graph
                .get(&label)
                .into_iter()
                .flatten()
                .filter_map(|target| cell_numbers.get(target).copied())
                .collect::<Vec<_>>();
            let enemy = enemy_nodes.get(&label);
            WikiwikiNodeDefinition {
                label: label.clone(),
                cell_no: cell_numbers[&label],
                is_boss: enemy.is_some_and(|node| node.is_boss),
                is_battle: enemy.is_some(),
                next_cells,
            }
        })
        .collect()
}

pub(super) fn parse_route_table(
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
            let row_target = row.get(to_idx).and_then(|cell| parse_node_label(cell))?;
            let raw_text = row
                .iter()
                .skip(cond_idx)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
            Some(((source_label, raw_text), row_target))
        })
        .fold(BTreeMap::<(String, String), BTreeSet<String>>::new(), |mut acc, (key, target)| {
            acc.entry(key).or_default().insert(target);
            acc
        });

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

pub(super) fn postprocess_route_probabilities(rules: &mut Vec<RouteRuleDraft>) {
    let source_targets =
        rules.iter().fold(BTreeMap::<String, BTreeSet<String>>::new(), |mut acc, rule| {
            acc.entry(rule.from_label.clone()).or_default().insert(rule.to_label.clone());
            acc
        });

    let mut additions = Vec::new();
    let mut derived_sources = BTreeSet::new();
    for (from_label, targets) in source_targets {
        if targets.len() != 2 {
            continue;
        }
        let source_rules = rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| rule.from_label == from_label)
            .collect::<Vec<_>>();
        let probability_target = source_rules
            .iter()
            .find(|(_, rule)| rule.probability_pct.is_some())
            .map(|(_, rule)| rule.to_label.clone());
        let placeholder_target = source_rules
            .iter()
            .find(|(_, rule)| rule.random_placeholder)
            .map(|(_, rule)| rule.to_label.clone());

        let (Some(probability_target), Some(placeholder_target)) =
            (probability_target, placeholder_target)
        else {
            continue;
        };
        if probability_target == placeholder_target {
            continue;
        }

        let derived = source_rules
            .iter()
            .filter_map(|(_, rule)| {
                rule.probability_pct.map(|pct| RouteRuleDraft {
                    from_label: rule.from_label.clone(),
                    to_label: placeholder_target.clone(),
                    probability_pct: Some((100.0 - pct).max(0.0)),
                    predicate: rule.predicate.clone(),
                    raw_text: format!("{} (derived complement)", rule.raw_text),
                    random_placeholder: false,
                })
            })
            .collect::<Vec<_>>();
        if !derived.is_empty() {
            derived_sources.insert(from_label);
        }
        additions.extend(derived);
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

pub(super) fn parse_independent_route_condition_line(
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

pub(super) fn parse_case_route_condition_text(
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

    let flush_group = |target: &Option<String>,
                       terms: &mut Vec<String>,
                       parsed: &mut Vec<(Option<f64>, RoutePredicate, bool)>|
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

pub(super) fn parse_inline_targeted_route_condition_text(
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

pub(super) fn parse_route_predicate(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    let mut text = sanitize_route_text(text);
    while let Some(trimmed) = trim_wrapping_parentheses(&text) {
        text = trimmed;
    }
    if text.is_empty() || text == "それ以外" {
        return Some(RoutePredicate::Always);
    }
    if text.contains("艦隊人数により確率変動") || text.contains("艦隊人数によって確率変動")
    {
        let weights: Vec<FleetSizeWeight> = RE_PROBABILITY
            .captures_iter(&text)
            .filter_map(|cap| {
                Some(FleetSizeWeight {
                    fleet_size: cap["count"].parse().ok()?,
                    probability_pct: cap["pct"].parse().ok()?,
                })
            })
            .collect();
        if !weights.is_empty() {
            return Some(RoutePredicate::FleetSizeWeightedRandom {
                weights,
            });
        }
    }
    if let Some((left, right)) = split_once_keyword_top_level(&text, "または")
        .or_else(|| split_once_keyword_top_level(&text, "もしくは"))
    {
        return Some(RoutePredicate::Or(vec![
            parse_route_predicate(left, ship_types, ships)?,
            parse_route_predicate(right, ship_types, ships)?,
        ]));
    }
    if let Some((left, right)) = split_once_keyword_top_level(&text, "かつ")
        .or_else(|| split_once_keyword_top_level(&text, "且つ"))
    {
        return Some(RoutePredicate::And(vec![
            parse_route_predicate(left, ship_types, ships)?,
            parse_route_predicate(right, ship_types, ships)?,
        ]));
    }
    if let Some(predicate) = parse_contains_conjunction(&text, ship_types, ships) {
        return Some(predicate);
    }
    if let Some(predicate) = parse_los_range_predicate(&text) {
        return Some(predicate);
    }
    if let Some(predicate) = parse_los_simple_predicate(&text) {
        return Some(predicate);
    }
    if let Some(predicate) = parse_visited_node_predicate(&text) {
        return Some(predicate);
    }
    if let Some(predicate) = parse_equipment_count_predicate(&text) {
        return Some(predicate);
    }
    if let Some(predicate) = parse_flagship_predicate(&text, ship_types, ships) {
        return Some(predicate);
    }
    if let Some(predicate) =
        parse_speed_qualified_selector_count_predicate(&text, ship_types, ships)
    {
        return Some(predicate);
    }

    if let Some(caps) = RE_LOS.captures(&text) {
        let formula = caps
            .name("formula")
            .map(|value| normalize_text(value.as_str()))
            .filter(|value| !value.is_empty());
        let value = caps.name("value")?.as_str().parse::<i64>().ok()?;
        return Some(RoutePredicate::LoS {
            formula,
            op: RouteOperator::Gte,
            value,
        });
    }

    if let Some(caps) = RE_DRUM_COUNT.captures(&text) {
        return Some(RoutePredicate::DrumCanisterCount {
            op: RouteOperator::Gte,
            value: caps.name("count")?.as_str().parse::<i64>().ok()?,
        });
    }
    if let Some(predicate) = parse_ship_type_contains_count_clause(&text, ship_types, ships) {
        return Some(predicate);
    }
    if let Some((selector, op, value)) = parse_ship_selector_count_clause(&text, ship_types, ships)
    {
        return Some(match (selector.ship_types.is_empty(), selector.ship_ids.is_empty()) {
            (false, true) => RoutePredicate::ShipTypeCount {
                ship_types: selector.ship_types,
                op,
                value,
            },
            (true, false) | (false, false) => RoutePredicate::ShipSetCount {
                ship_types: selector.ship_types,
                ship_ids: selector.ship_ids,
                op,
                value,
            },
            (true, true) => return None,
        });
    }

    if let Some(caps) = RE_FLEET_SIZE_LTE.captures(&text) {
        return Some(RoutePredicate::FleetSize {
            op: RouteOperator::Lte,
            value: caps.name("count")?.as_str().parse::<i64>().ok()?,
        });
    }
    if let Some(caps) = RE_FLEET_SIZE_GTE.captures(&text) {
        return Some(RoutePredicate::FleetSize {
            op: RouteOperator::Gte,
            value: caps.name("count")?.as_str().parse::<i64>().ok()?,
        });
    }
    if let Some(caps) = RE_FLEET_SIZE_EQ.captures(&text) {
        return Some(RoutePredicate::FleetSize {
            op: RouteOperator::Eq,
            value: caps.name("count")?.as_str().parse::<i64>().ok()?,
        });
    }

    if let Some(token) = text.strip_suffix("のみの艦隊") {
        if let Some(selector) = resolve_route_selector(token, ship_types, ships) {
            return predicate_for_only_selector(selector);
        }
        return ship_types.resolve(token).map(|ship_type| RoutePredicate::OnlyShipTypes {
            ship_types: vec![ship_type],
        });
    }
    if let Some(predicate) = parse_contains_predicate(&text, ship_types, ships) {
        return Some(predicate);
    }

    if text.contains("最速統一") {
        return Some(RoutePredicate::Speed {
            class: SpeedClass::Fastest,
        });
    }
    if text.contains("高速以上統一") {
        return Some(RoutePredicate::Speed {
            class: SpeedClass::Fast,
        });
    }
    if text.contains("高速+以上統一") || text.contains("高速+統一") {
        return Some(RoutePredicate::Speed {
            class: SpeedClass::FastPlus,
        });
    }
    if text.contains("高速+以上の統一") || text.contains("高速+の統一") {
        return Some(RoutePredicate::Speed {
            class: SpeedClass::FastPlus,
        });
    }
    if text.contains("高速+統一") {
        return Some(RoutePredicate::Speed {
            class: SpeedClass::Fast,
        });
    }
    if text.contains("高速統一") {
        return Some(RoutePredicate::Speed {
            class: SpeedClass::Fast,
        });
    }
    if text.contains("低速艦を含む")
        || text.contains("低速 を含む")
        || text.contains("速力:低速を含む")
        || text.contains("速力:低速 を含む")
        || text.contains("速力:低速を含み")
        || text.contains("速力:低速 を含み")
        || text.contains("速力:低速 が含まれている")
        || text.contains("低速 が含まれている")
    {
        return Some(RoutePredicate::Not(Box::new(RoutePredicate::Speed {
            class: SpeedClass::Fast,
        })));
    }

    if let Some(token) = text.strip_suffix("のみの艦隊")
        && let Some(ship_types) = ship_types.resolve_group(token)
    {
        return Some(RoutePredicate::OnlyShipTypes {
            ship_types,
        });
    }

    if let Some((ship_types, op, value)) = parse_ship_type_count_clause(&text, ship_types) {
        return Some(RoutePredicate::ShipTypeCount {
            ship_types,
            op,
            value,
        });
    }
    if let Some(ship_ids) = parse_specific_ship_id_list(&text, ships) {
        return Some(super::combine_ship_id_predicates(ship_ids, RoutePredicate::And));
    }

    if let Some(ship_ids) = parse_specific_ship_list(&text, ships) {
        return Some(RoutePredicate::ContainsShipId {
            ship_ids,
        });
    }
    ship_types.resolve_group(&text).map(|ship_types| RoutePredicate::ContainsShipType {
        ship_types,
    })
}

fn parse_contains_conjunction(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    for suffix in ["を含み", "含み", "を含む", "含む"] {
        let Some((left, right)) = text.split_once(suffix) else {
            continue;
        };
        let right = right.trim();
        if right.is_empty() {
            continue;
        }
        return Some(RoutePredicate::And(vec![
            parse_route_predicate(&format!("{left}{suffix}"), ship_types, ships)?,
            parse_route_predicate(right, ship_types, ships)?,
        ]));
    }
    None
}

fn parse_ship_type_contains_count_clause(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    let text = normalize_contains_count_clause_text(text);
    let caps = RE_SHIP_TYPE_CONTAINS_COUNT_CLAUSE.captures(&text)?;
    let token = caps.name("name")?.as_str().trim_end_matches("から");
    let selector = resolve_route_selector(token, ship_types, ships)?;
    let value = caps.name("count")?.as_str().parse::<i64>().ok()?;
    let positive = !text.ends_with("含まない");
    let op = match caps.name("op").map(|value| value.as_str()) {
        Some("以上") | None => RouteOperator::Gte,
        Some("以下") => RouteOperator::Lte,
        Some("ちょうど") | Some("過不足なく") => RouteOperator::Eq,
        Some(_) => return None,
    };
    let predicate = match (selector.ship_types.is_empty(), selector.ship_ids.is_empty()) {
        (false, true) => RoutePredicate::ShipTypeCount {
            ship_types: selector.ship_types,
            op,
            value,
        },
        (true, false) | (false, false) => RoutePredicate::ShipSetCount {
            ship_types: selector.ship_types,
            ship_ids: selector.ship_ids,
            op,
            value,
        },
        (true, true) => return None,
    };
    if positive {
        Some(predicate)
    } else {
        Some(RoutePredicate::Not(Box::new(predicate)))
    }
}

fn normalize_contains_count_clause_text(text: &str) -> String {
    let text = sanitize_route_text(text)
        .replace("(過不足なく)", "過不足なく")
        .replace("（過不足なく）", "過不足なく")
        .replace("(ちょうど)", "ちょうど")
        .replace("（ちょうど）", "ちょうど");
    if let Some(caps) = RE_CONTAINS_COUNT_OP_BEFORE_COUNT.captures(&text) {
        let name =
            normalize_text(caps.name("name").map(|value| value.as_str()).unwrap_or_default());
        let op = caps.name("op").map(|value| value.as_str()).unwrap_or_default();
        let count = caps.name("count").map(|value| value.as_str()).unwrap_or_default();
        let suffix = caps.name("suffix").map(|value| value.as_str()).unwrap_or_default();
        if !name.is_empty() && !count.is_empty() && !suffix.is_empty() {
            return format!("{name}{count}{op}を含{suffix}");
        }
    }
    text
}

fn parse_contains_predicate(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    for (suffix, negative) in [
        ("を含まない", true),
        ("含まない", true),
        ("を含み", false),
        ("含み", false),
        ("を含む", false),
        ("含む", false),
    ] {
        let Some(token) = text.strip_suffix(suffix).map(str::trim) else {
            continue;
        };
        if token.is_empty() {
            continue;
        }

        if let Some(predicate) = parse_named_pair_contains_predicate(token, negative, ships) {
            return Some(predicate);
        }
        if let Some(ship_ids) = parse_specific_ship_id_list(token, ships) {
            let predicate = super::combine_ship_id_predicates(ship_ids, RoutePredicate::And);
            return Some(if negative {
                RoutePredicate::Not(Box::new(predicate))
            } else {
                predicate
            });
        }

        if let Some(base) = token.strip_suffix("以外").map(str::trim) {
            let selector = resolve_route_selector(base, ship_types, ships)?;
            let predicate = predicate_for_only_selector(selector)?;
            return Some(RoutePredicate::Not(Box::new(predicate)));
        }

        if let Some(predicate) = parse_contains_selector_token(token, negative, ship_types, ships) {
            return Some(predicate);
        }

        let selector = resolve_route_selector(token, ship_types, ships)?;
        let predicate = predicate_for_contains_selector(selector)?;
        return Some(if negative {
            RoutePredicate::Not(Box::new(predicate))
        } else {
            predicate
        });
    }

    None
}

fn parse_contains_selector_token(
    token: &str,
    negative: bool,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    let parts = token
        .split(['、', ',', '，'])
        .map(sanitize_route_text)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() <= 1 {
        let predicate = parse_contains_selector_part(token, ship_types, ships)?;
        return Some(if negative {
            RoutePredicate::Not(Box::new(predicate))
        } else {
            predicate
        });
    }

    let predicates = parts
        .into_iter()
        .map(|part| parse_contains_selector_part(&part, ship_types, ships))
        .collect::<Option<Vec<_>>>()?;
    let predicate = if predicates.len() == 1 {
        predicates.into_iter().next().unwrap_or(RoutePredicate::Always)
    } else {
        RoutePredicate::Or(predicates)
    };
    Some(if negative {
        RoutePredicate::Not(Box::new(predicate))
    } else {
        predicate
    })
}

fn parse_contains_selector_part(
    token: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    let normalized = sanitize_route_text(token);
    let speed_qualifier = normalized
        .strip_prefix("(低速)")
        .or_else(|| normalized.strip_prefix("（低速）"))
        .or_else(|| normalized.strip_prefix("低速"))
        .map(|base| (RouteOperator::Lte, SpeedClass::Slow, base));

    if let Some((speed_op, speed_class, base)) = speed_qualifier {
        let selector = resolve_route_selector(base, ship_types, ships)?;
        return Some(RoutePredicate::ShipSetSpeedCount {
            ship_types: selector.ship_types,
            ship_ids: selector.ship_ids,
            speed_op,
            speed_class,
            op: RouteOperator::Gte,
            value: 1,
        });
    }

    let selector = resolve_route_selector(&normalized, ship_types, ships)?;
    predicate_for_contains_selector(selector)
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

fn split_once_keyword_top_level<'a>(text: &'a str, needle: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0_i32;
    for (idx, ch) in text.char_indices() {
        match ch {
            '(' | '（' => depth += 1,
            ')' | '）' => depth = (depth - 1).max(0),
            _ => {}
        }
        if depth == 0 && text[idx..].starts_with(needle) {
            let left = text[..idx].trim();
            let right = text[idx + needle.len()..].trim();
            if !left.is_empty() && !right.is_empty() {
                return Some((left, right));
            }
        }
    }
    None
}

fn trim_wrapping_parentheses(text: &str) -> Option<String> {
    let text = text.trim();
    let (open, close) = match (text.chars().next()?, text.chars().last()?) {
        ('(', ')') => ('(', ')'),
        ('（', '）') => ('（', '）'),
        _ => return None,
    };
    let mut depth = 0_i32;
    let chars = text.chars().collect::<Vec<_>>();
    for (idx, ch) in chars.iter().enumerate() {
        match ch {
            c if *c == open => depth += 1,
            c if *c == close => {
                depth -= 1;
                if depth == 0 && idx != chars.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }
    (depth == 0).then(|| text[open.len_utf8()..text.len() - close.len_utf8()].trim().to_string())
}

fn parse_los_range_predicate(text: &str) -> Option<RoutePredicate> {
    let caps = RE_LOS_RANGE
        .captures(text)
        .or_else(|| RE_LOS_RANGE_LTE.captures(text))
        .or_else(|| RE_LOS_RANGE_BARE.captures(text))
        .or_else(|| RE_LOS_RANGE_ALT.captures(text))
        .or_else(|| RE_LOS_RANGE_LTE_BARE.captures(text))?;
    let mut min = caps.name("min")?.as_str().parse::<i64>().ok()?;
    let mut max = caps.name("max")?.as_str().parse::<i64>().ok()?;
    if max < min {
        std::mem::swap(&mut min, &mut max);
    }
    if max <= min {
        return None;
    }
    Some(RoutePredicate::And(vec![
        RoutePredicate::LoS {
            formula: None,
            op: RouteOperator::Gte,
            value: min,
        },
        RoutePredicate::LoS {
            formula: None,
            op: RouteOperator::Lte,
            value: max - 1,
        },
    ]))
}

fn parse_los_simple_predicate(text: &str) -> Option<RoutePredicate> {
    if let Some(caps) = RE_LOS_GTE_SIMPLE.captures(text).or_else(|| RE_LOS_GTE_BARE.captures(text))
    {
        return Some(RoutePredicate::LoS {
            formula: None,
            op: RouteOperator::Gte,
            value: caps.name("value")?.as_str().parse::<i64>().ok()?,
        });
    }
    if let Some(caps) = RE_LOS_LTE_SIMPLE.captures(text).or_else(|| RE_LOS_LTE_BARE.captures(text))
    {
        return Some(RoutePredicate::LoS {
            formula: None,
            op: RouteOperator::Lte,
            value: caps.name("value")?.as_str().parse::<i64>().ok()?,
        });
    }
    if let Some(caps) = RE_LOS_LT_SIMPLE.captures(text).or_else(|| RE_LOS_LT_BARE.captures(text)) {
        return Some(RoutePredicate::LoS {
            formula: None,
            op: RouteOperator::Lte,
            value: caps.name("value")?.as_str().parse::<i64>().ok()?.saturating_sub(1),
        });
    }
    None
}

fn parse_visited_node_predicate(text: &str) -> Option<RoutePredicate> {
    if let Some(caps) = RE_VISITED_NODE_NEGATIVE.captures(text) {
        return Some(RoutePredicate::VisitedNodeLabel {
            node_labels: vec![caps.name("label")?.as_str().to_string()],
            visited: false,
        });
    }
    if let Some(caps) = RE_VISITED_NODE_POSITIVE.captures(text) {
        return Some(RoutePredicate::VisitedNodeLabel {
            node_labels: vec![caps.name("label")?.as_str().to_string()],
            visited: true,
        });
    }
    None
}

fn parse_equipment_count_predicate(text: &str) -> Option<RoutePredicate> {
    let caps = RE_EQUIPMENT_COUNT.captures(text)?;
    let slotitem_types = resolve_equipment_slotitem_types(caps.name("name")?.as_str())?;
    let value = caps.name("count")?.as_str().parse::<i64>().ok()?;
    let op = match caps.name("op").map(|value| value.as_str()) {
        Some("以上") => RouteOperator::Gte,
        Some("以下") => RouteOperator::Lte,
        Some(_) => return None,
        None => RouteOperator::Eq,
    };
    Some(RoutePredicate::EquipmentCount {
        slotitem_types,
        op,
        value,
    })
}

fn resolve_equipment_slotitem_types(text: &str) -> Option<Vec<i64>> {
    match sanitize_route_text(text).as_str() {
        "電探" => Some(vec![12, 13, 93]),
        "小型電探" => Some(vec![12]),
        "大型電探" | "大型電探II" | "大型電探（II）" => Some(vec![13, 93]),
        _ => None,
    }
}

fn parse_flagship_predicate(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    let caps = RE_FLAGSHIP.captures(text)?;
    let token = sanitize_route_text(caps.name("name")?.as_str());
    if let Some(ship_types) = ship_types.resolve_group(&token) {
        return Some(RoutePredicate::FlagshipShipType {
            ship_types,
        });
    }
    if let Some(ship_ids) = parse_specific_ship_list(&token, ships) {
        return Some(RoutePredicate::FlagshipShipId {
            ship_ids,
        });
    }
    None
}

fn parse_speed_qualified_selector_count_predicate(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    let caps = RE_SPEED_QUALIFIED_COUNT.captures(text)?;
    let (speed_op, speed_class) =
        parse_speed_qualified_selector_prefix(caps.name("speed")?.as_str())?;
    let selector = resolve_route_selector(caps.name("name")?.as_str(), ship_types, ships)?;
    let value = caps.name("count")?.as_str().parse::<i64>().ok()?;
    let op = match caps.name("op").map(|value| value.as_str()) {
        Some("以上") => RouteOperator::Gte,
        Some("以下") => RouteOperator::Lte,
        Some("ちょうど") | Some("過不足なく") | None => RouteOperator::Eq,
        Some(_) => return None,
    };
    Some(RoutePredicate::ShipSetSpeedCount {
        ship_types: selector.ship_types,
        ship_ids: selector.ship_ids,
        speed_op,
        speed_class,
        op,
        value,
    })
}

fn parse_speed_qualified_selector_prefix(text: &str) -> Option<(RouteOperator, SpeedClass)> {
    match sanitize_route_text(text).as_str() {
        "低速" | "(低速)" | "（低速）" => Some((RouteOperator::Lte, SpeedClass::Slow)),
        "高速" => Some((RouteOperator::Gte, SpeedClass::Fast)),
        "高速+" | "高速＋" => Some((RouteOperator::Gte, SpeedClass::FastPlus)),
        "最速" => Some((RouteOperator::Gte, SpeedClass::Fastest)),
        _ => None,
    }
}

pub(super) fn parse_target_random_route_condition_text(
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

pub(super) fn parse_row_target_random_bias_condition_text(
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

pub(super) fn parse_row_target_random_bias_shorthand_condition_text(
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

pub(super) fn parse_conditional_random_route_condition_text(
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
