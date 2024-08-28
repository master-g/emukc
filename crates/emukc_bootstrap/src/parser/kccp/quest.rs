use std::collections::BTreeMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::parser::error::ParseError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KccpQuestInfo {
	pub id: i64,
	pub name: String,
	pub desc: String,
}

impl Default for KccpQuestInfo {
	fn default() -> Self {
		Self {
			id: 0,
			name: "n/a".to_string(),
			desc: "n/a".to_string(),
		}
	}
}

enum ParserStatus {
	Id,
	Name,
	Desc,
}

/// Parse the quest info from the raw string.
///
/// # Arguments
///
/// * `raw` - The raw string to parse.
///
/// # Returns
///
/// A map of quest id to quest info.
pub fn parse(raw: &str) -> Result<BTreeMap<i64, KccpQuestInfo>, ParseError> {
	let reg_id = Regex::new(r"_quest_id_(\d+)").unwrap();
	let reg_desc = Regex::new(r#""([^"]+)""#).unwrap();

	let mut status = ParserStatus::Id;
	let mut result: BTreeMap<i64, KccpQuestInfo> = BTreeMap::new();

	let mut quest_id: Option<i64> = None;
	let mut quest_name: Option<String> = None;

	for line in raw.lines() {
		match status {
			ParserStatus::Id => {
				if let Some(caps) = reg_id.captures(line) {
					if let Some(matched) = caps.get(1) {
						quest_id = Some(matched.as_str().parse().unwrap());
						status = ParserStatus::Name;
					}
				}
			}
			ParserStatus::Name => {
				// 	"【節分任務:鬼】南西方面節分作戦二〇二四": "[Setsubun] Southwestern Area Setsubun Operation 2024",
				if let Some(name) = line.split("\":").next() {
					quest_name = Some(name.trim_start().replace('"', ""));
					status = ParserStatus::Desc;
				}
			}
			ParserStatus::Desc => {
				let mut matches = reg_desc.captures_iter(line);
				if let Some(cap) = matches.next() {
					let desc = cap
						.get(1)
						.unwrap()
						.as_str()
						.trim_start()
						.replace('"', "")
						.replace("\\n", "<br>");

					if let (Some(id), Some(name)) = (quest_id, &quest_name) {
						result.insert(
							id,
							KccpQuestInfo {
								id,
								name: name.clone(),
								desc,
							},
						);
					}

					status = ParserStatus::Id;
				}
			}
		}
	}

	Ok(result)
}
