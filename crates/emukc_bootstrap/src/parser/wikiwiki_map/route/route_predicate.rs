use std::sync::LazyLock;

use emukc_model::codex::map::{FleetSizeWeight, RouteOperator, RoutePredicate, SpeedClass};
use regex::Regex;

use super::super::{
    ShipResolver, ShipTypeResolver, combine_ship_id_predicates, normalize_text,
    parse_named_pair_contains_predicate, parse_ship_selector_count_clause,
    parse_ship_type_count_clause, parse_specific_ship_id_list, parse_specific_ship_list,
    predicate_for_contains_selector, predicate_for_only_selector, resolve_route_selector,
    sanitize_route_text,
};

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
static RE_SHIP_TYPE_CONTAINS_COUNT_CLAUSE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
		r"^(?P<name>.+?)(?P<count>\d+)隻(?P<op>以上|以下|ちょうど|過不足なく)?を含(?:む|み|まない)$",
	)
	.expect("valid ship type contains count clause regex")
});
static RE_EQUIPMENT_COUNT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<name>.+?)(?:を装備した艦|搭載艦の隻数)が(?P<count>\d+)隻?(?P<op>以上|以下)?$")
        .expect("valid equipment count regex")
});

pub fn unknown_predicate(raw_text: String) -> RoutePredicate {
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

pub fn compact_route_raw_text(predicate: &RoutePredicate, raw_text: String) -> String {
    if matches!(predicate, RoutePredicate::Unknown { .. } | RoutePredicate::SourceUnknown { .. }) {
        raw_text
    } else {
        String::new()
    }
}

pub fn parse_route_predicate(
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
        return Some(combine_ship_id_predicates(ship_ids, RoutePredicate::And));
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
    super::super::types::normalize_count_clause_text(text, Some("む"))
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
            let predicate = combine_ship_id_predicates(ship_ids, RoutePredicate::And);
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
