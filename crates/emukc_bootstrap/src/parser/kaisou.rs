use std::path::Path;

use emukc_model::prelude::*;
use regex::Regex;

use super::error::ParseError;

fn extract_from_start2_json(manifest: &ApiManifest) -> Vec<KcShipRemodelRequirement> {
	manifest
		.api_mst_ship
		.iter()
		.filter_map(|ship| {
			let sortno = ship.api_sortno.as_ref()?;
			let after_ship_id = ship.api_aftershipid.as_ref()?;
			if *sortno == 0 || after_ship_id.is_empty() || after_ship_id == "0" {
				return None;
			}
			let after_ship_id: i64 = if let Ok(after_ship_id) = after_ship_id.parse() {
				after_ship_id
			} else {
				return None;
			};

			Some(KcShipRemodelRequirement {
				id_from: ship.api_id,
				id_to: after_ship_id,
				ammo: ship.api_afterbull.unwrap_or(0),
				steel: ship.api_afterfuel.unwrap_or(0),
				drawing: 0,
				catapult: 0,
				report: 0,
				devmat: 0,
				torch: 0,
				aviation: 0,
				artillery: 0,
				arms: 0,
				boiler: 0,
			})
		})
		.flat_map(|mut info| {
			let upgrade = manifest.api_mst_shipupgrade.iter().find(|upgrade| {
				upgrade.api_current_ship_id == info.id_from && upgrade.api_id == info.id_to
			});
			if let Some(upgrade) = upgrade {
				info.drawing = upgrade.api_drawing_count;
				info.catapult = upgrade.api_catapult_count;
				info.report = upgrade.api_report_count;
				info.aviation = upgrade.api_aviation_mat_count;
				info.artillery = upgrade.api_arms_mat_count;
				info.boiler = upgrade.api_boiler_count.unwrap_or(0);
			}
			Some(info)
		})
		.collect()
}

fn extract_hokoheso_from_main_js(
	content: &str,
	data: &mut [KcShipRemodelRequirement],
) -> Result<(), ParseError> {
	let regex_pattern =
		Regex::new(r"return (\d+) == this\.mst_id_before \? (\d+) : (\d+);").unwrap();
	let mut found = false;
	let mut ship_ids: Vec<i64> = vec![];

	for line in content.lines() {
		if !found && line.contains(".prototype, 'newhokohesosizai', {") {
			found = true;
			continue;
		}
		if found {
			if line.contains("case ") {
				let ship_id = line
					.split_whitespace()
					.last()
					.unwrap()
					.trim_end_matches(':')
					.parse::<i64>()
					.unwrap();
				ship_ids.push(ship_id);
			} else if line.contains("return ") {
				if line.contains("==") {
					if let Some(captures) = regex_pattern.captures(line) {
						let from_id: i64 = captures[1].parse().unwrap();
						let cost_if_matched: i64 = captures[2].parse().unwrap();
						let not_matched: i64 = captures[3].parse().unwrap();

						for info in data.iter_mut().filter(|info| ship_ids.contains(&info.id_to)) {
							if info.id_from == from_id {
								info.artillery = cost_if_matched;
							} else {
								info.artillery = not_matched;
							}
						}
					} else {
						error!("No match found for special case: {}", line);
					}
				} else {
					let newhokohesosizai = line
						.split_whitespace()
						.last()
						.unwrap()
						.trim_end_matches(';')
						.parse::<i64>()
						.unwrap();

					if !ship_ids.is_empty() {
						ship_ids.iter().for_each(|id| {
							if let Some(info) = data.iter_mut().find(|info| info.id_to == *id) {
								info.artillery = newhokohesosizai;
							} else {
								error!("Failed to find ship_id: {} for newhokohesosizai", id);
							}
						});
						ship_ids.clear();
					}
				}
			} else if line.contains("default:") {
				break;
			}
		}
	}

	Ok(())
}

fn extract_use_devkit_group(raw: &str) -> Vec<i64> {
	let rex = Regex::new(r#"this._USE_DEVKIT_GROUP_ = \[((?:\d+(?:, )?)+)\];"#).unwrap();

	if let Some(captures) = rex.captures(raw) {
		let numbers = captures.get(1).unwrap().as_str();
		let result: Vec<i64> = numbers.split(", ").map(|num| num.parse().unwrap()).collect();
		// trace!("use_devkit_group: {:?}", result);
		result
	} else {
		vec![]
	}
}

fn extract_devkit(content: &str, data: &mut [KcShipRemodelRequirement]) {
	let group = extract_use_devkit_group(content);
	if group.is_empty() {
		error!("Failed to extract devkit group");
		return;
	}

	let mut found = false;
	let mut ship_ids: Vec<i64> = vec![];
	let mut marked: Vec<i64> = vec![];

	let rex_ternary = Regex::new(r#"_f4j < (\d+) \? (\d+) : "#).unwrap();
	let rex_fallback = Regex::new(r#"(\d+);"#).unwrap();
	let mut mappings = Vec::new();

	for line in content.lines() {
		if !found && line.contains("_getRequiredDevkitNum = function(") {
			found = true;
			continue;
		}
		if found {
			if line.contains("case ") {
				let ship_id = line
					.split_whitespace()
					.last()
					.unwrap()
					.trim_end_matches(':')
					.parse::<i64>()
					.unwrap();
				ship_ids.push(ship_id);
				marked.push(ship_id);
			} else if line.contains("return ") {
				let devkit_num = line
					.split_whitespace()
					.last()
					.unwrap()
					.trim_end_matches(';')
					.parse::<i64>()
					.unwrap();

				if !ship_ids.is_empty() {
					ship_ids.iter().for_each(|id| {
						if let Some(info) = data.iter_mut().find(|info| info.id_from == *id) {
							info.devmat = devkit_num;
						}
					});
					ship_ids.clear();
				}
			}
			if line.contains("return 0 !=") {
				for captures in rex_ternary.captures_iter(line) {
					let range_val: i64 = captures.get(1).unwrap().as_str().parse().unwrap();
					let return_val: i64 = captures.get(2).unwrap().as_str().parse().unwrap();

					mappings.push((range_val, return_val));
				}

				if let Some(captures) = rex_fallback.captures(line) {
					let fallback_val: i64 = captures.get(1).unwrap().as_str().parse().unwrap();

					mappings.push((i64::MAX, fallback_val));
				}

				if mappings.is_empty() {
					error!("Failed to extract default branch for mapping");
					return;
				}

				let first_range = mappings.first().unwrap().0;
				let first_value = mappings.first().unwrap().1;
				data.iter_mut().filter(|info| !marked.contains(&info.id_from)).for_each(|info| {
					if info.drawing != 0 && !group.contains(&info.id_from)
						|| info.steel < first_range
					{
						info.devmat = first_value;
					} else {
						info.devmat = mappings
							.iter()
							.skip(1)
							.find(|(range_val, _)| info.steel < *range_val)
							.map(|(_, return_val)| *return_val)
							.unwrap_or(0);
					}
				});

				break;
			}
		}
	}
}

fn extract_buildkit(content: &str, data: &mut [KcShipRemodelRequirement]) {
	let mut ship_id: Vec<i64> = vec![];
	let mut found = false;

	for line in content.lines() {
		if !found && line.contains("_getRequiredBuildKitNum = function(") {
			found = true;
			continue;
		}
		if found {
			if line.contains("case ") {
				let id = line
					.split_whitespace()
					.last()
					.unwrap()
					.trim_end_matches(':')
					.parse::<i64>()
					.unwrap();
				ship_id.push(id);
			} else if line.contains("return ") {
				let num = line
					.split_whitespace()
					.last()
					.unwrap()
					.trim_end_matches(';')
					.parse::<i64>()
					.unwrap();
				ship_id.iter().for_each(|id| {
					if let Some(info) = data.iter_mut().find(|info| info.id_from == *id) {
						info.torch = num;
					}
				});
				ship_id.clear();
			} else if line.contains("default:") {
				break;
			}
		}
	}
}

fn extract_from_main_js(
	path: impl AsRef<Path>,
	data: &mut [KcShipRemodelRequirement],
) -> Result<(), ParseError> {
	let content = std::fs::read_to_string(path)?;
	extract_hokoheso_from_main_js(&content, data)?;
	extract_devkit(&content, data);
	extract_buildkit(&content, data);

	Ok(())
}

/// Parse kaisou data extracted from start2.json and main.js
///
/// # Arguments
///
/// * `src` - Path to the `main.js`
/// * `manifest` - The api manifest
pub fn parse(
	src: impl AsRef<Path>,
	manifest: &ApiManifest,
) -> Result<KcShipRemodelRequirementMap, ParseError> {
	trace!("extracting kaisou data from start2.json");
	let mut data = extract_from_start2_json(manifest);
	trace!("{} records extracted", data.len());

	trace!("extracting kaisou data from main.js");
	extract_from_main_js(src, &mut data)?;

	Ok(data.into_iter().map(|info| ((info.id_from, info.id_to), info)).collect())
}
