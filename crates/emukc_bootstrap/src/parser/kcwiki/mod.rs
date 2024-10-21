//! `KCWiki` data parser

use emukc_model::prelude::{ApiManifest, Kc3rdSlotItemExtraInfoMap};

use super::error::ParseError;

mod db_slotitem;
mod ship;
mod slotitem;

#[derive(Debug, Clone, PartialEq)]
struct ParseContext {
	slotitem_name_map: std::collections::BTreeMap<String, i64>,

	ship_name_map: std::collections::BTreeMap<String, i64>,
}

impl ParseContext {
	pub fn find_slotitem_id(&self, name: &str) -> Option<i64> {
		self.slotitem_name_map.get(name).copied()
	}

	pub fn find_ship_id(&self, name: &str) -> Option<i64> {
		self.ship_name_map.get(name).copied()
	}
}

fn prepare_context(src: impl AsRef<std::path::Path>) -> Result<ParseContext, ParseError> {
	let slotitem_json_path = src.as_ref().join("kcwiki_slotitem.json");
	let ship_json_path = src.as_ref().join("kcwiki_ship.json");

	let slotitem_name_map = slotitem::parse_slotitem_name_mapping(&slotitem_json_path)?;
	let ship_name_map = ship::parse_ship_name_mapping(&ship_json_path)?;

	Ok(ParseContext {
		slotitem_name_map,
		ship_name_map,
	})
}

pub fn parse(
	src: impl AsRef<std::path::Path>,
	_manifest: &ApiManifest,
) -> Result<Kc3rdSlotItemExtraInfoMap, ParseError> {
	let context = prepare_context(&src)?;
	slotitem::parse(&context, &src)?;
	ship::parse(&context, &src)?;

	let db_slotitems_path = src.as_ref().join("kcwiki_db_slotitem.json");
	db_slotitem::parse(&db_slotitems_path)
}

#[cfg(test)]
mod tests {
	use crate::parser::kcwiki::slotitem;

	fn get_parse_context() -> super::ParseContext {
		let pwd = std::env::current_dir().unwrap();
		println!("current dir: {:?}", pwd);

		let src = std::path::Path::new("../../.data/temp");
		super::prepare_context(src).unwrap()
	}

	#[test_log::test]
	fn test_parse() {
		let context = get_parse_context();
		let map = slotitem::parse(
			&context,
			std::path::Path::new("../../.data/temp/kcwiki_slotitem.json"),
		)
		.unwrap();
		println!("slotitem: {}", map.map.len());
	}
}
