//! KCanotify expedition data parser

use std::collections::HashMap;
use std::path::Path;

use super::types::KCanotifyExpedition;
use crate::parser::error::ParseError;
use emukc_model::thirdparty::{
	Kc3rdCompositionAlternative, Kc3rdDrumRequirements, Kc3rdExpeditionCondition,
	Kc3rdExpeditionConditionMap, Kc3rdExpeditionItemReward, Kc3rdExpeditionName,
	Kc3rdExpeditionRequirements, Kc3rdShipTypeRequirement,
};

/// Parse KCanotify expedition data file
pub fn parse(path: impl AsRef<Path>) -> Result<Kc3rdExpeditionConditionMap, ParseError> {
	let raw = std::fs::read_to_string(path)?;
	let kcanotify_data: Vec<KCanotifyExpedition> = serde_json::from_str(&raw)?;

	let mut seen_ids = HashMap::new();
	let mut duplicates = Vec::new();

	let result: Kc3rdExpeditionConditionMap = kcanotify_data
		.into_iter()
		.filter_map(|expedition| {
			let condition = match convert_to_internal_model(&expedition) {
				Ok(c) => c,
				Err(e) => return Some(Err(e)),
			};

			if let Some(existing_code) = seen_ids.insert(condition.api_id, expedition.code.clone())
			{
				duplicates.push((condition.api_id, existing_code, expedition.code));
				return None;
			}

			Some(Ok((condition.api_id, condition)))
		})
		.collect::<Result<HashMap<_, _>, ParseError>>()?;

	if !duplicates.is_empty() {
		let dup_str = duplicates
			.iter()
			.map(|(id, old, new)| format!("  id={}: '{}' and '{}'", id, old, new))
			.collect::<Vec<_>>()
			.join("\n");
		return Err(ParseError::Generic(format!("Found duplicate expedition IDs:\n{}", dup_str)));
	}

	Ok(result)
}

/// Convert KCanotify data to internal model
fn convert_to_internal_model(
	src: &KCanotifyExpedition,
) -> Result<Kc3rdExpeditionCondition, ParseError> {
	let api_id = src.id.parse::<i64>().map_err(|e| ParseError::IntParse(e.to_string()))?;

	Ok(Kc3rdExpeditionCondition {
		api_id,
		code: src.code.clone(),
		area: src.area,
		name: convert_name(&src.name),
		time_minutes: src.time,
		resource_reward: src.resource,
		item_rewards: convert_rewards(&src.reward),
		admiral_exp: src.exp[0],
		fleet_exp: src.exp[1],
		requirements: Kc3rdExpeditionRequirements {
			ship_count: src.total_num,
			flagship_level: src.flagship_level,
			fleet_level: src.total_level,
			flagship_type: parse_flagship_type(&src.flagship_type),
			composition: parse_composition(&src.total_condition)?,
			total_firepower: src.total_firepower.or(src.total_fp),
			total_asw: src.total_asw,
			total_los: src.total_los,
			drum_requirements: parse_drum_requirements(
				src.drum_ship,
				src.drum_num,
				src.drum_num_optional,
			),
		},
	})
}

/// Convert multilingual names
fn convert_name(src: &super::types::KCanotifyExpeditionName) -> Kc3rdExpeditionName {
	Kc3rdExpeditionName {
		ja: src.jp.clone(),
		ko: src.ko.clone(),
		en: src.en.clone(),
		zh_cn: src.scn.clone(),
		zh_tw: src.tcn.clone(),
	}
}

/// Convert item rewards
fn convert_rewards(rewards: &[[i64; 2]]) -> Vec<Kc3rdExpeditionItemReward> {
	rewards
		.iter()
		.filter(|r| r[0] != 0)
		.map(|r| Kc3rdExpeditionItemReward {
			item_id: r[0],
			count: r[1],
		})
		.collect()
}

/// Parse flagship type
fn parse_flagship_type(cond: &Option<String>) -> Option<i64> {
	cond.as_ref().and_then(|s| s.parse().ok())
}

/// Parse composition condition expression
///
/// Format: "ship_type-count|ship_type,ship_type-count/..."
/// - `/` separates OR conditions
/// - `|` separates AND conditions within an OR branch
/// - `,` separates alternative ship types for a single requirement
///
/// Examples:
/// - "3-1" -> 1 light cruiser
/// - "3-1|2-2" -> 1 light cruiser AND 2 destroyers
/// - "3-1/2-2" -> 1 light cruiser OR 2 destroyers
/// - "1,2-3" -> 3 ships of any type OR destroyer
fn parse_composition(
	cond: &Option<String>,
) -> Result<Vec<Kc3rdCompositionAlternative>, ParseError> {
	let Some(cond_str) = cond else {
		return Ok(vec![]);
	};

	if cond_str.is_empty() {
		return Ok(vec![]);
	}

	cond_str
		.split('/')
		.map(|alt_str| {
			let conditions = alt_str
				.split('|')
				.map(|and_str| parse_ship_type_requirement(and_str.trim()))
				.collect::<Result<Vec<_>, _>>()?;
			Ok(Kc3rdCompositionAlternative {
				conditions,
			})
		})
		.collect()
}

/// Parse single ship type requirement
///
/// Format: "ship_types-count" where ship_types can be comma-separated
/// Example: "1,2-3" means 3 ships of type 1 OR type 2
fn parse_ship_type_requirement(s: &str) -> Result<Kc3rdShipTypeRequirement, ParseError> {
	let (types_part, count_part) = s.split_once('-').ok_or_else(|| {
		ParseError::Generic(format!("Invalid ship type requirement format: {}", s))
	})?;

	let ship_types = types_part
		.split(',')
		.map(|t| t.parse::<i64>())
		.collect::<Result<Vec<_>, _>>()
		.map_err(|e| ParseError::IntParse(e.to_string()))?;

	let count = count_part.parse::<i64>().map_err(|e| ParseError::IntParse(e.to_string()))?;

	Ok(Kc3rdShipTypeRequirement {
		ship_types,
		count,
	})
}

/// Parse drum requirements
///
/// Handles both required drums (ship_count + total_count) and optional drums (optional_count)
fn parse_drum_requirements(
	ship_count: Option<i64>,
	total_count: Option<i64>,
	optional_count: Option<i64>,
) -> Option<Kc3rdDrumRequirements> {
	let (ships, total, optional) = match (ship_count, total_count, optional_count) {
		(Some(ships), Some(total), _) => (ships, total, false),
		(None, None, Some(opt)) => (1, opt, true),
		_ => return None,
	};

	Some(Kc3rdDrumRequirements {
		ship_count: ships,
		total_count: total,
		optional,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_composition_simple() {
		let result = parse_composition(&Some("3-1".to_string())).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].conditions.len(), 1);
		assert_eq!(result[0].conditions[0].ship_types, vec![3]);
		assert_eq!(result[0].conditions[0].count, 1);
	}

	#[test]
	fn test_parse_composition_and() {
		let result = parse_composition(&Some("3-1|2-2".to_string())).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].conditions.len(), 2);
		assert_eq!(result[0].conditions[0].ship_types, vec![3]);
		assert_eq!(result[0].conditions[0].count, 1);
		assert_eq!(result[0].conditions[1].ship_types, vec![2]);
		assert_eq!(result[0].conditions[1].count, 2);
	}

	#[test]
	fn test_parse_composition_or() {
		let result = parse_composition(&Some("3-1/2-2".to_string())).unwrap();
		assert_eq!(result.len(), 2);
		// First alternative: 1 light cruiser (type 3)
		assert_eq!(result[0].conditions[0].ship_types, vec![3]);
		assert_eq!(result[0].conditions[0].count, 1);
		// Second alternative: 2 destroyers (type 2)
		assert_eq!(result[1].conditions[0].ship_types, vec![2]);
		assert_eq!(result[1].conditions[0].count, 2);
	}

	#[test]
	fn test_parse_composition_multi_ship_types() {
		// "3-1|1,2-2" -> 1 light cruiser AND 2 ships of type 1/2
		let result = parse_composition(&Some("3-1|1,2-2".to_string())).unwrap();
		assert_eq!(result.len(), 1);
		assert_eq!(result[0].conditions[0].ship_types, vec![3]);
		assert_eq!(result[0].conditions[1].ship_types, vec![1, 2]);
		assert_eq!(result[0].conditions[1].count, 2);
	}

	#[test]
	fn test_parse_composition_multi_branch_or() {
		// OR condition: "2-2/3-1" -> 2 destroyers OR 1 light cruiser
		let result = parse_composition(&Some("2-2/3-1".to_string())).unwrap();
		assert_eq!(result.len(), 2);
		assert_eq!(result[0].conditions[0].ship_types, vec![2]);
		assert_eq!(result[0].conditions[0].count, 2);
		assert_eq!(result[1].conditions[0].ship_types, vec![3]);
		assert_eq!(result[1].conditions[0].count, 1);
	}

	#[test]
	fn test_parse_composition_empty() {
		assert!(parse_composition(&None).unwrap().is_empty());
		assert!(parse_composition(&Some("".to_string())).unwrap().is_empty());
	}

	#[test]
	fn test_parse_composition_invalid_format() {
		assert!(parse_composition(&Some("invalid".to_string())).is_err());
		assert!(parse_composition(&Some("1-2-3".to_string())).is_err());
	}

	#[test]
	fn test_parse_drum_requirements_required() {
		let result = parse_drum_requirements(Some(3), Some(4), None);
		assert!(result.is_some());
		let req = result.unwrap();
		assert_eq!(req.ship_count, 3);
		assert_eq!(req.total_count, 4);
		assert!(!req.optional);
	}

	#[test]
	fn test_parse_drum_requirements_optional() {
		let result = parse_drum_requirements(None, None, Some(3));
		assert!(result.is_some());
		let req = result.unwrap();
		assert_eq!(req.ship_count, 1);
		assert_eq!(req.total_count, 3);
		assert!(req.optional);
	}

	#[test]
	fn test_parse_drum_requirements_none() {
		assert!(parse_drum_requirements(None, None, None).is_none());
		assert!(parse_drum_requirements(Some(1), None, None).is_none());
		assert!(parse_drum_requirements(None, Some(1), None).is_none());
	}

	#[test]
	fn test_parse_ship_type_requirement_invalid() {
		assert!(parse_ship_type_requirement("invalid").is_err());
		assert!(parse_ship_type_requirement("1-2-3").is_err());
		assert!(parse_ship_type_requirement("abc-def").is_err());
	}

	#[test]
	fn test_convert_rewards_filters_zero() {
		let rewards = vec![[0, 0], [1, 2], [0, 0], [3, 4]];
		let result = convert_rewards(&rewards);
		assert_eq!(result.len(), 2);
		assert_eq!(result[0].item_id, 1);
		assert_eq!(result[0].count, 2);
		assert_eq!(result[1].item_id, 3);
		assert_eq!(result[1].count, 4);
	}

	#[test]
	fn test_parse_flagship_type() {
		assert_eq!(parse_flagship_type(&Some("3".to_string())), Some(3));
		assert_eq!(parse_flagship_type(&Some("invalid".to_string())), None);
		assert_eq!(parse_flagship_type(&None), None);
	}

	#[test]
	fn test_parse_detects_duplicate_ids() {
		use std::io::Write;
		use tempfile::NamedTempFile;

		let json = r#"[
			{
				"no": "1",
				"code": "EXP-001",
				"area": 1,
				"name": {"jp": "Test1", "en": "Test1", "scn": "Test1", "tcn": "Test1", "ko": "Test1"},
				"time": 30,
				"resource": [100, 100, 100, 100],
				"reward": [[1, 2]],
				"exp": [100, 200],
				"flagship": "1",
				"flag-lv": null,
				"flag-type": null,
				"flag-num": null,
				"total-num": 1,
				"total-lv": null,
				"total-cond": null,
				"total-firepower": null,
				"total-asw": null,
				"total-los": null,
				"drum-ship": null,
				"drum-num": null,
				"drum-num-optional": null
			},
			{
				"no": "1",
				"code": "EXP-002",
				"area": 2,
				"name": {"jp": "Test2", "en": "Test2", "scn": "Test2", "tcn": "Test2", "ko": "Test2"},
				"time": 45,
				"resource": [200, 200, 200, 200],
				"reward": [[2, 3]],
				"exp": [150, 250],
				"flagship": "1",
				"flag-lv": null,
				"flag-type": null,
				"flag-num": null,
				"total-num": 2,
				"total-lv": null,
				"total-cond": null,
				"total-firepower": null,
				"total-asw": null,
				"total-los": null,
				"drum-ship": null,
				"drum-num": null,
				"drum-num-optional": null
			}
		]"#;

		let mut temp_file = NamedTempFile::new().unwrap();
		temp_file.write_all(json.as_bytes()).unwrap();

		let result = parse(temp_file.path());
		assert!(result.is_err());
		let err_msg = result.unwrap_err().to_string();
		assert!(err_msg.contains("duplicate expedition IDs"));
		assert!(err_msg.contains("EXP-001"));
		assert!(err_msg.contains("EXP-002"));
	}
}
