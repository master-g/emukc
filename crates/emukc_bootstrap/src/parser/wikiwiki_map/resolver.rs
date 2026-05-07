use std::collections::{BTreeMap, BTreeSet};

use emukc_model::{
    codex::map::{RouteOperator, RoutePredicate},
    kc2::start2::{ApiManifest, ApiMstShip},
};

use super::{
    RE_PAREN_ANNOTATION, RE_SHIP_TYPE_COUNT_CLAUSE, RouteSelector, ShipResolver, ShipTypeResolver,
    normalize_text, sanitize_route_text,
};

impl ShipTypeResolver {
    pub(super) fn new(manifest: &ApiManifest) -> Self {
        let mut aliases = BTreeMap::new();
        let mut groups = BTreeMap::<String, Vec<i64>>::new();
        for ship_type in &manifest.api_mst_stype {
            let name = normalize_text(&ship_type.api_name);
            if !name.is_empty() {
                aliases.insert(name.clone(), ship_type.api_id);
                groups.insert(name, vec![ship_type.api_id]);
            }
        }
        for (alias, canonical) in [
            ("海防", "海防艦"),
            ("駆逐", "駆逐艦"),
            ("軽巡", "軽巡洋艦"),
            ("練巡", "練習巡洋艦"),
            ("雷巡", "重雷装巡洋艦"),
            ("重巡", "重巡洋艦"),
            ("航巡", "航空巡洋艦"),
            ("軽空母", "軽空母"),
            ("正規空母", "正規空母"),
            ("戦艦", "戦艦"),
            ("高速戦艦", "高速戦艦"),
            ("航空戦艦", "航空戦艦"),
            ("潜水艦", "潜水艦"),
            ("潜水空母", "潜水空母"),
            ("補給", "補給艦"),
            ("補給艦", "補給艦"),
            ("水母", "水上機母艦"),
            ("揚陸艦", "揚陸艦"),
            ("工作艦", "工作艦"),
        ] {
            if let Some(id) = aliases.get(canonical).copied() {
                aliases.insert(alias.to_string(), id);
                groups.insert(alias.to_string(), vec![id]);
            }
        }
        for (alias, canonicals) in [
            ("空母系", vec!["軽空母", "正規空母", "装甲空母"]),
            ("戦艦級", vec!["戦艦", "高速戦艦", "航空戦艦"]),
            ("重巡級", vec!["重巡洋艦", "航空巡洋艦"]),
            ("軽巡級", vec!["軽巡洋艦", "練習巡洋艦", "重雷装巡洋艦"]),
        ] {
            let ids = canonicals
                .into_iter()
                .filter_map(|name| aliases.get(name).copied())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            if !ids.is_empty() {
                groups.insert(alias.to_string(), ids);
            }
        }
        Self {
            aliases,
            groups,
        }
    }

    pub(super) fn resolve(&self, raw: &str) -> Option<i64> {
        let token = normalize_text(raw);
        self.aliases.get(&token).copied().or_else(|| {
            token.strip_suffix('艦').and_then(|trimmed| self.aliases.get(trimmed).copied())
        })
    }

    pub(super) fn resolve_group(&self, raw: &str) -> Option<Vec<i64>> {
        let token = normalize_group_token(raw);
        if token.is_empty() {
            return None;
        }
        if token.contains('+') {
            let ids = token
                .split('+')
                .filter_map(|part| self.resolve_group(part))
                .flatten()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            return (!ids.is_empty()).then_some(ids);
        }
        self.groups.get(&token).cloned().or_else(|| self.resolve(&token).map(|id| vec![id]))
    }
}

impl ShipResolver {
    pub(super) fn new(manifest: &ApiManifest) -> Self {
        let mut labels = BTreeMap::new();
        let mut class_groups = BTreeMap::<String, Vec<i64>>::new();
        let class_members = manifest.api_mst_ship.iter().fold(
            BTreeMap::<i64, BTreeSet<i64>>::new(),
            |mut acc, ship| {
                acc.entry(ship.api_ctype).or_default().insert(ship.api_id);
                acc
            },
        );
        for ship in &manifest.api_mst_ship {
            for label in candidate_ship_labels(ship) {
                for key in ship_lookup_keys(&label) {
                    labels.entry(key).or_insert(ship.api_id);
                }
            }
            let class_key = normalize_text(&format!("{}型", ship.api_name));
            if !class_key.is_empty()
                && let Some(ids) = class_members.get(&ship.api_ctype)
            {
                class_groups.entry(class_key).or_insert_with(|| ids.iter().copied().collect());
            }
        }
        Self {
            labels,
            class_groups,
        }
    }

    pub(super) fn resolve(&self, raw: &str) -> Option<i64> {
        ship_lookup_keys(raw).into_iter().find_map(|token| self.labels.get(&token).copied())
    }

    pub(super) fn extract_all(&self, raw: &str) -> Vec<(i64, String)> {
        let mut remaining = normalize_text(raw);
        let mut matches = Vec::new();

        while !remaining.is_empty() {
            if let Some((matched, ship_id)) = self.longest_prefix_match(&remaining) {
                matches.push((ship_id, matched.clone()));
                remaining = remaining[matched.len()..]
                    .trim_start_matches([' ', '、', ',', '／', '/', '・'])
                    .trim()
                    .to_string();
                continue;
            }

            let Some(idx) = remaining.find([' ', '、', ',', '／', '/', '・']) else {
                break;
            };
            remaining = remaining[idx + 1..].trim().to_string();
        }

        matches
    }

    pub(super) fn resolve_class_group(&self, raw: &str) -> Option<Vec<i64>> {
        let token = normalize_group_token(raw);
        self.class_groups.get(&token).cloned()
    }

    fn longest_prefix_match(&self, raw: &str) -> Option<(String, i64)> {
        let mut best: Option<(String, i64)> = None;
        for (label, ship_id) in &self.labels {
            if !raw.starts_with(label) {
                continue;
            }
            let next = raw[label.len()..].chars().next();
            if next.is_some_and(|ch| !matches!(ch, ' ' | '、' | ',' | '／' | '/' | '・')) {
                continue;
            }
            let replace = best.as_ref().is_none_or(|(current, _)| label.len() > current.len());
            if replace {
                best = Some((label.clone(), *ship_id));
            }
        }
        best
    }
}

pub(super) fn candidate_ship_labels(ship: &ApiMstShip) -> Vec<String> {
    let mut labels = BTreeSet::new();
    let base = normalize_text(&ship.api_name);
    if !base.is_empty() {
        labels.insert(base.clone());
    }
    let suffix = normalize_enemy_suffix(&ship.api_yomi);
    if !suffix.is_empty() && !base.is_empty() {
        labels.insert(format!("{base}{suffix}"));
        labels.insert(format!("{base} {suffix}"));
    }
    labels.into_iter().collect()
}

pub(super) fn normalize_enemy_suffix(raw: &str) -> String {
    match raw.trim().trim_matches('-') {
        "" => String::new(),
        value => value.to_string(),
    }
}

pub(super) fn ship_lookup_keys(raw: &str) -> Vec<String> {
    let normalized = normalize_text(raw);
    let trimmed = normalized.trim_end_matches('?').to_string();
    let stripped = normalize_text(&RE_PAREN_ANNOTATION.replace_all(&trimmed, ""));
    let mut keys = Vec::new();
    let mut seen = BTreeSet::new();

    for candidate in [
        trimmed.as_str(),
        trimmed.replace(' ', "").as_str(),
        stripped.as_str(),
        stripped.replace(' ', "").as_str(),
    ] {
        let candidate = normalize_text(candidate);
        if !candidate.is_empty() && seen.insert(candidate.clone()) {
            keys.push(candidate);
        }
    }

    keys
}

pub(super) fn normalize_group_token(raw: &str) -> String {
    let token =
        sanitize_route_text(raw).trim_matches(|c| matches!(c, '(' | ')' | '（' | '）')).to_string();
    normalize_text(&token)
}

pub(super) fn parse_ship_type_count_clause(
    text: &str,
    ship_types: &ShipTypeResolver,
) -> Option<(Vec<i64>, RouteOperator, i64)> {
    let text = normalize_count_clause_text(text);
    let caps = RE_SHIP_TYPE_COUNT_CLAUSE.captures(&text)?;
    let token = caps.name("name")?.as_str();
    let ship_types = ship_types.resolve_group(token)?;
    let count = caps.name("count")?.as_str().parse::<i64>().ok()?;
    let op = match caps.name("op").map(|value| value.as_str()) {
        Some("以上") => RouteOperator::Gte,
        Some("以下") => RouteOperator::Lte,
        Some("ちょうど") | Some("過不足なく") | None => RouteOperator::Eq,
        Some(_) => return None,
    };
    Some((ship_types, op, count))
}

pub(super) fn parse_ship_selector_count_clause(
    text: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<(RouteSelector, RouteOperator, i64)> {
    let text = normalize_count_clause_text(text);
    let caps = RE_SHIP_TYPE_COUNT_CLAUSE.captures(&text)?;
    let token = caps.name("name")?.as_str().trim_end_matches("から");
    let selector = resolve_route_selector(token, ship_types, ships)?;
    if selector.ship_types.is_empty() && selector.ship_ids.is_empty() {
        return None;
    }
    let count = caps.name("count")?.as_str().parse::<i64>().ok()?;
    let op = match caps.name("op").map(|value| value.as_str()) {
        Some("以上") => RouteOperator::Gte,
        Some("以下") => RouteOperator::Lte,
        Some("ちょうど") | Some("過不足なく") | None => RouteOperator::Eq,
        Some(_) => return None,
    };
    Some((selector, op, count))
}

pub(super) fn normalize_count_clause_text(text: &str) -> String {
    super::types::normalize_count_clause_text(text, None)
}

pub(super) fn parse_specific_ship_list(text: &str, ships: &ShipResolver) -> Option<Vec<i64>> {
    let ship_id = ships.resolve(text)?;
    Some(vec![ship_id])
}

pub(super) fn parse_specific_ship_id_list(text: &str, ships: &ShipResolver) -> Option<Vec<i64>> {
    let ship_ids = text
        .split('と')
        .map(normalize_text)
        .filter(|part| !part.is_empty())
        .map(|part| ships.resolve(&part))
        .collect::<Option<Vec<_>>>()?;
    let ship_ids = ship_ids.into_iter().collect::<BTreeSet<_>>().into_iter().collect::<Vec<_>>();
    (ship_ids.len() >= 2).then_some(ship_ids)
}

pub(super) fn combine_ship_id_predicates(
    ship_ids: Vec<i64>,
    combine: fn(Vec<RoutePredicate>) -> RoutePredicate,
) -> RoutePredicate {
    let predicates = ship_ids
        .into_iter()
        .map(|ship_id| RoutePredicate::ContainsShipId {
            ship_ids: vec![ship_id],
        })
        .collect::<Vec<_>>();
    if predicates.len() == 1 {
        predicates.into_iter().next().unwrap_or(RoutePredicate::Always)
    } else {
        combine(predicates)
    }
}

pub(super) fn parse_named_pair_contains_predicate(
    token: &str,
    negative: bool,
    ships: &ShipResolver,
) -> Option<RoutePredicate> {
    if let Some(base) = token.strip_suffix("の両方").map(str::trim)
        && let Some(ship_ids) = parse_specific_ship_id_list(base, ships)
    {
        let predicate = combine_ship_id_predicates(ship_ids, RoutePredicate::And);
        return Some(if negative {
            RoutePredicate::Not(Box::new(predicate))
        } else {
            predicate
        });
    }
    if let Some(base) =
        token.strip_suffix("のいずれも").or_else(|| token.strip_suffix("のどちらも")).map(str::trim)
        && let Some(ship_ids) = parse_specific_ship_id_list(base, ships)
    {
        let predicate = combine_ship_id_predicates(ship_ids, RoutePredicate::Or);
        return Some(if negative {
            RoutePredicate::Not(Box::new(predicate))
        } else {
            predicate
        });
    }
    None
}

pub(super) fn resolve_route_selector(
    raw: &str,
    ship_types: &ShipTypeResolver,
    ships: &ShipResolver,
) -> Option<RouteSelector> {
    let token = normalize_group_token(raw);
    if token.is_empty() {
        return None;
    }

    let parts = token
        .split(['+', '＋', '、', ',', '，'])
        .map(normalize_group_token)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }

    let mut selector = RouteSelector::default();
    let mut seen_type = BTreeSet::new();
    let mut seen_ship = BTreeSet::new();
    for part in parts {
        if let Some(ship_type_ids) = ship_types.resolve_group(&part) {
            for ship_type in ship_type_ids {
                if seen_type.insert(ship_type) {
                    selector.ship_types.push(ship_type);
                }
            }
            continue;
        }
        if let Some(group_ship_ids) = ships.resolve_class_group(&part) {
            for ship_id in group_ship_ids {
                if seen_ship.insert(ship_id) {
                    selector.ship_ids.push(ship_id);
                }
            }
            continue;
        }
        if let Some(ship_ids) = parse_specific_ship_list(&part, ships) {
            for ship_id in ship_ids {
                if seen_ship.insert(ship_id) {
                    selector.ship_ids.push(ship_id);
                }
            }
            continue;
        }
        return None;
    }

    (!selector.ship_types.is_empty() || !selector.ship_ids.is_empty()).then_some(selector)
}

pub(super) fn predicate_for_contains_selector(selector: RouteSelector) -> Option<RoutePredicate> {
    match (selector.ship_types.is_empty(), selector.ship_ids.is_empty()) {
        (false, true) => Some(RoutePredicate::ContainsShipType {
            ship_types: selector.ship_types,
        }),
        (true, false) if selector.ship_ids.len() == 1 => Some(RoutePredicate::ContainsShipId {
            ship_ids: selector.ship_ids,
        }),
        (true, false) | (false, false) => Some(RoutePredicate::ContainsShipSet {
            ship_types: selector.ship_types,
            ship_ids: selector.ship_ids,
        }),
        (true, true) => None,
    }
}

pub(super) fn predicate_for_only_selector(selector: RouteSelector) -> Option<RoutePredicate> {
    match (selector.ship_types.is_empty(), selector.ship_ids.is_empty()) {
        (false, true) => Some(RoutePredicate::OnlyShipTypes {
            ship_types: selector.ship_types,
        }),
        (true, false) | (false, false) => Some(RoutePredicate::OnlyShipSet {
            ship_types: selector.ship_types,
            ship_ids: selector.ship_ids,
        }),
        (true, true) => None,
    }
}
