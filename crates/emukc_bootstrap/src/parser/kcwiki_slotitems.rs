use std::path::Path;

use emukc_model::prelude::*;
use serde::{Deserialize, Serialize};

use super::error::ParseError;

/// Parse the slot item extra info.
///
/// # Arguments
///
/// * `src` - The source directory.
/// * `manifest` - The API manifest.
pub fn parse(src: impl AsRef<Path>) -> Result<Kc3rdSlotItemExtraInfoMap, ParseError> {
	let src = src.as_ref();
	trace!("parsing slot item extra info: {:?}", src);

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
