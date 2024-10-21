use std::{collections::BTreeMap, path::Path};

use serde::{Deserialize, Serialize};

use crate::parser::error::ParseError;

use super::ParseContext;

/// Parse the `kcwiki_slotitem.json` first-pass for EN name to `mst_id` mapping.
pub(super) fn parse_ship_name_mapping(
	src: impl AsRef<Path>,
) -> Result<BTreeMap<String, i64>, ParseError> {
	let src = src.as_ref();
	trace!("parsing kcwiki ship for name mapping: {:?}", src);

	#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
	struct Entry {
		#[serde(rename = "_id")]
		id: i64,

		#[serde(rename = "_full_name")]
		name: String,
	}

	let map: BTreeMap<String, Entry> = serde_json::from_reader(std::fs::File::open(src)?)?;

	for (k, v) in map.iter() {
		if k != &v.name {
			error!("{} != {}", k, v.name);
		}
	}

	let map = map.into_iter().map(|(k, v)| (k, v.id)).collect();

	Ok(map)
}

pub(super) fn parse(context: &ParseContext, src: impl AsRef<Path>) -> Result<(), ParseError> {
	todo!();
}
