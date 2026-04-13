use std::collections::BTreeMap;

use emukc_model::codex::map::ShipDropDefinition;
use scraper::{ElementRef, node::Node};

use super::{
    DropCellEvent, SELECTOR_CELL, SELECTOR_ROW, ShipDropDraft, ShipResolver, extract_cell_text,
    normalize_text, parse_node_label, sanitize_drop_text,
};

pub(super) fn parse_drop_table(
    map_name: &str,
    table: &ElementRef<'_>,
    ships: &ShipResolver,
    warnings: &mut Vec<String>,
) -> Vec<ShipDropDraft> {
    let mut drops = Vec::new();

    for row in table.select(&SELECTOR_ROW) {
        let cells = row.select(&SELECTOR_CELL).collect::<Vec<_>>();
        if cells.len() < 2 {
            continue;
        }

        let row_label = extract_cell_text(&cells[0]);
        let Some(node_label) = parse_node_label(&row_label) else {
            continue;
        };

        for cell in cells.into_iter().skip(1) {
            for drop in extract_ship_drops_from_cell(map_name, &node_label, &cell, ships, warnings)
            {
                drops.push(ShipDropDraft {
                    node_label: node_label.clone(),
                    drop,
                });
            }
        }
    }

    drops
}

fn extract_ship_drops_from_cell(
    map_name: &str,
    node_label: &str,
    cell: &ElementRef<'_>,
    ships: &ShipResolver,
    warnings: &mut Vec<String>,
) -> Vec<ShipDropDefinition> {
    let events = collect_drop_cell_events(cell, &[]);
    let mut drops = Vec::new();

    for event in events {
        let DropCellEvent::Text {
            text,
            tags,
        } = event
        else {
            continue;
        };
        let text = sanitize_drop_text(&text);
        if text.is_empty() {
            continue;
        }

        let matches = ships.extract_all(&text);
        for (ship_id, raw_ship_name) in &matches {
            drops.push(ShipDropDefinition {
                ship_id: *ship_id,
                raw_ship_name: raw_ship_name.clone(),
                tags: tags.clone(),
            });
        }

        if !text.trim().is_empty() && !is_drop_cell_placeholder(&text) && matches.is_empty() {
            warnings.push(format!(
                "unresolved drop cell `{}` in {} node {}",
                text.trim(),
                map_name,
                node_label
            ));
        }
    }

    let mut unique = BTreeMap::<(i64, String), ShipDropDefinition>::new();
    for drop in drops {
        unique
            .entry((drop.ship_id, drop.raw_ship_name.clone()))
            .and_modify(|existing| {
                for tag in &drop.tags {
                    if !existing.tags.contains(tag) {
                        existing.tags.push(tag.clone());
                    }
                }
            })
            .or_insert(drop);
    }
    unique.into_values().collect()
}

fn collect_drop_cell_events(
    cell: &ElementRef<'_>,
    inherited_tags: &[String],
) -> Vec<DropCellEvent> {
    let mut events = Vec::new();
    for child in cell.children() {
        match child.value() {
            Node::Text(text) => {
                let normalized = normalize_text(text);
                if !normalized.is_empty() {
                    events.push(DropCellEvent::Text {
                        text: normalized,
                        tags: inherited_tags.to_vec(),
                    });
                }
            }
            Node::Element(element) => {
                if element.name() == "br" {
                    events.push(DropCellEvent::Break);
                    continue;
                }

                let Some(child_ref) = ElementRef::wrap(child) else {
                    continue;
                };
                let mut next_tags = inherited_tags.to_vec();
                for tag in drop_tags_for_element(&child_ref) {
                    if !next_tags.contains(&tag) {
                        next_tags.push(tag);
                    }
                }
                events.extend(collect_drop_cell_events(&child_ref, &next_tags));
                if matches!(element.name(), "p" | "li") {
                    events.push(DropCellEvent::Break);
                }
            }
            _ => {}
        }
    }
    events
}

pub(super) fn drop_tags_for_element(element: &ElementRef<'_>) -> Vec<String> {
    let style = element.value().attr("style").unwrap_or_default().to_ascii_lowercase();
    let classes = element.value().attr("class").unwrap_or_default().to_ascii_lowercase();
    let mut tags = Vec::new();

    if style.contains("color:red") || style.contains("color: red") {
        tags.push("rare".to_string());
    }
    if style.contains("color:blue") || style.contains("color: blue") {
        tags.push("limited".to_string());
    }
    if classes.contains("wikicolor") && tags.is_empty() {
        if style.contains("red") {
            tags.push("rare".to_string());
        }
        if style.contains("blue") {
            tags.push("limited".to_string());
        }
    }

    tags
}

pub(super) fn is_drop_cell_placeholder(text: &str) -> bool {
    let normalized = sanitize_drop_text(text);
    normalized.is_empty()
        || normalized.chars().all(|ch| matches!(ch, '-' | '?' | '×'))
        || normalized.contains("background-color:gray")
}
