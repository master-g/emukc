use std::sync::LazyLock;

use emukc_model::codex::map::EnemyComposition;
use regex::Regex;
use scraper::Html;

use super::super::{
    RouteTableSection, SELECTOR_FOLD_CONTAINER, SELECTOR_TABLE, direct_child_tables,
    direct_child_with_class, normalize_text, table_to_grid,
};

pub fn find_route_table_sections(document: &Html) -> Vec<RouteTableSection> {
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
            super::super::find_table_by_headers(document, &["分岐点", "ルート", "移動条件"])
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

pub fn parse_gauge_defeat_counts(document: &Html) -> Vec<i64> {
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

pub fn route_section_variant_key(summary: &str, idx: usize, total: usize) -> String {
    if total <= 1 {
        return String::new();
    }
    let summary_compact = summary.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    if summary_compact.contains("Pマス出現前") {
        "pre_p_unlock".to_string()
    } else if summary_compact.contains("Pマス出現後") {
        "post_p_unlock".to_string()
    } else {
        // Match gauge keywords: 第一ゲージ/ゲージ1, 第二ゲージ/ゲージ2, etc.
        // Also handles 第三/第四 (kanji) and ゲージ3/4 (arabic).
        let gauge_key = extract_gauge_variant_key(&summary_compact);
        gauge_key.unwrap_or_else(|| format!("variant_{}", idx + 1))
    }
}

fn extract_gauge_variant_key(compact: &str) -> Option<String> {
    let kanji_map = [
        ("第一ゲージ", "gauge_1"),
        ("第二ゲージ", "gauge_2"),
        ("第三ゲージ", "gauge_3"),
        ("第四ゲージ", "gauge_4"),
        ("第五ゲージ", "gauge_5"),
    ];
    for (needle, key) in &kanji_map {
        if compact.contains(needle) {
            return Some(key.to_string());
        }
    }
    // Arabic numeral fallback: ゲージ1, ゲージ2, etc.
    for (needle, key) in [("ゲージ1", "gauge_1"), ("ゲージ2", "gauge_2")] {
        if compact.contains(needle) {
            return Some(key.to_string());
        }
    }
    // Regex fallback for ゲージN where N >= 3
    if let Some(pos) = compact.find("ゲージ") {
        let after = &compact[pos + "ゲージ".len()..];
        let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            return Some(format!("gauge_{digits}"));
        }
    }
    None
}

pub fn collect_formations(compositions: &[EnemyComposition]) -> Vec<i64> {
    use std::collections::BTreeSet;

    let mut formations = compositions
        .iter()
        .filter_map(|composition| composition.formation)
        .collect::<BTreeSet<_>>();
    if formations.is_empty() {
        formations.insert(1);
    }
    formations.into_iter().collect()
}
