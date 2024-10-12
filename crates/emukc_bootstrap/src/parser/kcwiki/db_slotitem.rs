use std::path::Path;

use emukc_model::prelude::*;
use serde::{Deserialize, Serialize};

use crate::parser::error::ParseError;

/// Parse the slot item extra info.
///
/// # Arguments
///
/// * `src` - The source directory.
pub fn parse(src: impl AsRef<Path>) -> Result<Kc3rdSlotItemExtraInfoMap, ParseError> {
	let src = src.as_ref();
	trace!("parsing kcwiki db slotitems info: {:?}", src);

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct Item {
		id: i64,
		name: String,
		description: String,
		// firepower: i64,
		// torpedo: i64,
		// armor: i64,
		// aa: i64,
		// bombing: i64,
		// evasion: i64,
		// asw: i64,
		// los: i64,
		// range: i64,
	}

	let items: Vec<Item> = serde_json::from_reader(std::fs::File::open(src)?)?;

	let mut map = Kc3rdSlotItemExtraInfoMap::new();
	for item in items {
		map.insert(
			item.id,
			Kc3rdSlotItemExtraInfo {
				api_id: item.id,
				info: item.description,
			},
		);
	}

	Ok(map)
}

#[cfg(test)]
mod tests {
	#[test]
	fn test_parse() {
		let src = std::path::Path::new("../../.data/temp/kcwiki_db_slotitem.json");
		let map = super::parse(src).unwrap();
		println!("{:?}", map);
	}
}
