use std::collections::BTreeMap;

use regex::Regex;
use scraper::{ElementRef, Html};

use super::{
    ENTRY_NODE_LABEL, RE_NODE_LABEL, SELECTOR_CELL, SELECTOR_FOLD_CONTAINER, SELECTOR_ROW,
    SELECTOR_TABLE, normalize_text,
};

pub(super) fn direct_child_with_class<'a>(
    element: &ElementRef<'a>,
    class_name: &str,
) -> Option<ElementRef<'a>> {
    element
        .children()
        .filter_map(ElementRef::wrap)
        .find(|child| element_has_class(child, class_name))
}

pub(super) fn direct_child_tables<'a>(element: &ElementRef<'a>) -> Vec<ElementRef<'a>> {
    let mut tables = Vec::new();
    for child in element.children().filter_map(ElementRef::wrap) {
        if child.value().name() == "table" {
            tables.push(child);
            continue;
        }
        tables.extend(
            child
                .children()
                .filter_map(ElementRef::wrap)
                .filter(|grandchild| grandchild.value().name() == "table"),
        );
    }
    tables
}

pub(super) fn element_has_class(element: &ElementRef<'_>, class_name: &str) -> bool {
    element
        .value()
        .attr("class")
        .is_some_and(|classes| classes.split_whitespace().any(|candidate| candidate == class_name))
}

pub(super) fn find_drop_table<'a>(document: &'a Html) -> Option<ElementRef<'a>> {
    document
        .select(&SELECTOR_FOLD_CONTAINER)
        .find_map(|container| {
            let summary = direct_child_with_class(&container, "fold-summary")
                .map(|summary| normalize_text(&summary.text().collect::<Vec<_>>().join(" ")))
                .unwrap_or_default();
            if !summary.contains("ドロップ") {
                return None;
            }
            let content = direct_child_with_class(&container, "fold-content")?;
            direct_child_tables(&content)
                .into_iter()
                .find(|table| is_drop_table_rows(&table_to_grid(table)))
        })
        .or_else(|| {
            document.select(&SELECTOR_TABLE).find(|table| is_drop_table_rows(&table_to_grid(table)))
        })
}

pub(super) fn is_drop_table_rows(rows: &[Vec<String>]) -> bool {
    let haystack = rows.iter().take(2).flat_map(|row| row.iter()).cloned().collect::<Vec<_>>();
    let known_headers =
        ["戦艦級", "航空母艦", "重巡級", "軽巡級", "駆逐艦", "海防艦", "潜水艦", "補助艦艇"];
    known_headers
        .iter()
        .filter(|header| haystack.iter().any(|cell| cell.contains(**header)))
        .count()
        >= 2
}

pub(super) fn find_table_by_headers(document: &Html, headers: &[&str]) -> Option<Vec<Vec<String>>> {
    document
        .select(&SELECTOR_TABLE)
        .filter_map(|table| {
            let grid = table_to_grid(&table);
            let haystack = grid
                .iter()
                .take(3)
                .flat_map(|row| row.iter())
                .cloned()
                .collect::<Vec<_>>()
                .join(" ");
            headers.iter().all(|header| haystack.contains(header)).then_some(grid)
        })
        .next()
}

pub(super) fn table_to_grid(table: &ElementRef<'_>) -> Vec<Vec<String>> {
    let mut pending = BTreeMap::<usize, (usize, String)>::new();
    let mut rows = Vec::new();

    for row in table.select(&SELECTOR_ROW) {
        let cells = row.select(&SELECTOR_CELL).collect::<Vec<_>>();
        if cells.is_empty() {
            continue;
        }

        let mut cols = Vec::new();
        let mut col_idx = 0_usize;
        for cell in cells {
            while let Some((remaining, text)) = pending.remove(&col_idx) {
                cols.push(text.clone());
                if remaining > 1 {
                    pending.insert(col_idx, (remaining - 1, text));
                }
                col_idx += 1;
            }

            let text = extract_cell_text(&cell);
            let rowspan = parse_span_attr(&cell, "rowspan");
            let colspan = parse_span_attr(&cell, "colspan");
            for offset in 0..colspan {
                cols.push(text.clone());
                if rowspan > 1 {
                    pending.insert(col_idx + offset, (rowspan - 1, text.clone()));
                }
            }
            col_idx += colspan;
        }

        while let Some((remaining, text)) = pending.remove(&col_idx) {
            cols.push(text.clone());
            if remaining > 1 {
                pending.insert(col_idx, (remaining - 1, text));
            }
            col_idx += 1;
        }

        if cols.iter().any(|cell| !cell.is_empty()) {
            rows.push(cols);
        }
    }

    rows
}

pub(super) fn extract_cell_text(cell: &ElementRef<'_>) -> String {
    static RE_BR_TAGS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<br[^>]*>").expect("valid br tag regex"));
    static RE_HTML_TAGS: std::sync::LazyLock<Regex> =
        std::sync::LazyLock::new(|| Regex::new(r"<[^>]+>").expect("valid html tag regex"));
    let html = cell.inner_html();
    let html = RE_BR_TAGS
        .replace_all(&html, "\n")
        .into_owned()
        .replace("</li>", "\n")
        .replace("</p>", "\n");
    let text = RE_HTML_TAGS.replace_all(&html, "");
    let mut lines =
        text.lines().map(normalize_text).filter(|line| !line.is_empty()).collect::<Vec<_>>();
    if lines.is_empty() {
        normalize_text(&text)
    } else {
        std::mem::take(&mut lines).join("\n")
    }
}

pub(super) fn parse_span_attr(cell: &ElementRef<'_>, attr: &str) -> usize {
    cell.value()
        .attr(attr)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

pub(super) fn parse_node_label(text: &str) -> Option<String> {
    let normalized = normalize_text(text)
        .trim_matches(|ch: char| matches!(ch, ':' | '：' | '→' | '-' | ' '))
        .to_string();
    if matches!(normalized.as_str(), "出撃" | "出撃ポイント" | "スタート")
        || normalized.eq_ignore_ascii_case(ENTRY_NODE_LABEL)
    {
        return Some(ENTRY_NODE_LABEL.to_string());
    }
    RE_NODE_LABEL
        .captures(&normalized)
        .and_then(|caps| caps.get(1))
        .map(|value| value.as_str().to_string())
}

pub(super) fn parse_formation(text: &str) -> Option<i64> {
    if text.contains("単縦") {
        Some(1)
    } else if text.contains("複縦") {
        Some(2)
    } else if text.contains("輪形") {
        Some(3)
    } else if text.contains("梯形") {
        Some(4)
    } else if text.contains("単横") {
        Some(5)
    } else if text.contains("警戒") {
        Some(6)
    } else {
        None
    }
}

pub(super) fn find_header_index(headers: &[String], names: &[&str]) -> Option<usize> {
    headers.iter().position(|header| names.iter().any(|name| header.contains(name)))
}
