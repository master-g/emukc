use std::collections::BTreeMap;

use emukc_model::codex::map::EnemyComposition;

use super::{
	EnemyNodeRows, RE_MULTIPLIER, ShipResolver, find_header_index, normalize_text, parse_formation,
	parse_node_label, parse_same_pattern_alias,
};
use crate::parser::error::ParseError;

pub(super) fn parse_enemy_table(
	map_name: &str,
	rows: &[Vec<String>],
	ships: &ShipResolver,
	warnings: &mut Vec<String>,
) -> Result<BTreeMap<String, EnemyNodeRows>, ParseError> {
	let header_idx = rows
		.iter()
		.position(|row| {
			row.iter().any(|cell| cell.contains("出現場所"))
				&& row.iter().any(|cell| cell.contains("出現艦船"))
		})
		.ok_or_else(|| ParseError::Generic("enemy header row not found".to_string()))?;
	let headers = &rows[header_idx];
	let node_idx = find_header_index(headers, &["出現場所"])
		.ok_or_else(|| ParseError::Generic("enemy table missing `出現場所` column".to_string()))?;
	let pattern_idx = find_header_index(headers, &["パターン"])
		.ok_or_else(|| ParseError::Generic("enemy table missing `パターン` column".to_string()))?;
	let ships_idx = find_header_index(headers, &["出現艦船"])
		.ok_or_else(|| ParseError::Generic("enemy table missing `出現艦船` column".to_string()))?;
	let formation_idx = find_header_index(headers, &["陣形"]);

	let mut nodes = BTreeMap::<String, EnemyNodeRows>::new();
	let mut pattern_aliases = BTreeMap::<(String, String), EnemyComposition>::new();
	for row in rows.iter().skip(header_idx + 1) {
		let Some(node_text) = row.get(node_idx) else {
			continue;
		};
		let Some(node_label) = parse_node_label(node_text) else {
			continue;
		};
		let pattern = row.get(pattern_idx).cloned().unwrap_or_else(|| "pattern".to_string());
		let normalized_pattern = normalize_text(&pattern);
		let formation =
			formation_idx.and_then(|idx| row.get(idx)).and_then(|text| parse_formation(text));
		if formation.is_none() && normalized_pattern.is_empty() {
			continue;
		}
		let ship_names =
			split_ship_names(row.get(ships_idx).map(String::as_str).unwrap_or_default());
		if ship_names.is_empty() {
			continue;
		}

		if ship_names.len() == 1
			&& let Some(alias_pattern) = parse_same_pattern_alias(&ship_names[0])
			&& let Some(previous) =
				pattern_aliases.get(&(node_label.clone(), alias_pattern.clone())).cloned()
		{
			let mut composition = previous;
			composition.comp_id = format!("{map_name}:{node_label}:{pattern}");
			composition.formation = formation.or(composition.formation);
			nodes
				.entry(node_label.clone())
				.or_insert_with(|| EnemyNodeRows {
					is_boss: node_text.contains("ボス"),
					compositions: Vec::new(),
				})
				.compositions
				.push(composition.clone());
			pattern_aliases.insert((node_label, normalized_pattern), composition);
			continue;
		}

		let mut ship_ids = Vec::new();
		for ship_name in &ship_names {
			let Some(ship_id) = ships.resolve(ship_name) else {
				warnings
					.push(format!("unresolved ship `{ship_name}` in {map_name} node {node_label}"));
				continue;
			};
			ship_ids.push(ship_id);
		}
		if ship_ids.is_empty() {
			continue;
		}

		let composition = EnemyComposition {
			comp_id: format!("{map_name}:{node_label}:{pattern}"),
			weight: 1,
			ship_ids,
			formation,
			raw_ship_names: ship_names,
		};
		nodes
			.entry(node_label.clone())
			.or_insert_with(|| EnemyNodeRows {
				is_boss: node_text.contains("ボス"),
				compositions: Vec::new(),
			})
			.compositions
			.push(composition.clone());
		pattern_aliases.insert((node_label, normalized_pattern), composition);
	}

	Ok(nodes)
}

pub(super) fn split_ship_names(text: &str) -> Vec<String> {
	text.split(['、', ',', '／', '/'])
		.flat_map(|entry| {
			let entry = normalize_text(entry);
			if entry.is_empty() {
				return Vec::new();
			}
			if let Some(caps) = RE_MULTIPLIER.captures(&entry)
				&& let (Some(name), Some(count)) = (caps.name("name"), caps.name("count"))
				&& let Ok(count) = count.as_str().parse::<usize>()
			{
				return std::iter::repeat_n(normalize_text(name.as_str()), count)
					.collect::<Vec<_>>();
			}
			vec![entry]
		})
		.collect()
}
