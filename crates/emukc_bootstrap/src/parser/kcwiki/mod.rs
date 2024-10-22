//! `KCWiki` data parser

use emukc_model::prelude::{ApiManifest, Kc3rdSlotItemMap};

use super::error::ParseError;

mod ship;
mod slot_item;
mod types;
mod use_item;

#[derive(Debug, Clone, PartialEq)]
struct ParseContext {
	slotitem_name_map: std::collections::BTreeMap<String, i64>,
	useitem_name_map: std::collections::BTreeMap<String, i64>,
	ship_name_map: std::collections::BTreeMap<String, i64>,
}

impl ParseContext {
	pub fn find_slotitem_id(&self, name: &str) -> Option<i64> {
		if let Some(&id) = self.slotitem_name_map.get(name) {
			return Some(id);
		}

		let after = if let Some(stripped) = name.strip_suffix('*') {
			if let Some(&id) = self.slotitem_name_map.get(stripped) {
				return Some(id);
			}
			stripped
		} else {
			name
		};

		let after = if after.contains("Ni") {
			let after = after.replace("Ni", "2");
			if let Some(&id) = self.slotitem_name_map.get(&after) {
				return Some(id);
			}
			after
		} else {
			after.to_owned()
		};

		if after.contains('_') {
			let after = after.replace('_', " ");
			if let Some(&id) = self.slotitem_name_map.get(&after) {
				return Some(id);
			}
		}

		None
	}

	pub fn find_ship_id(&self, name: &str) -> Option<i64> {
		if let Some(&id) = self.ship_name_map.get(name) {
			return Some(id);
		}

		let after = if let Some(stripped) = name.strip_suffix('/') {
			if let Some(&id) = self.ship_name_map.get(stripped) {
				return Some(id);
			}
			stripped
		} else {
			name
		};

		let after = if after.contains('/') {
			let after = after.replace('/', " ");
			if let Some(&id) = self.ship_name_map.get(&after) {
				return Some(id);
			}
			after
		} else {
			after.to_owned()
		};

		if after.contains("Carrier") {
			let after = after.replace("Carrier", "Kou");
			if let Some(&id) = self.ship_name_map.get(&after) {
				return Some(id);
			}
		};

		None
	}

	pub fn find_useitem_id(&self, name: &str) -> Option<i64> {
		self.useitem_name_map.get(name).copied()
	}
}

fn prepare_context(src: impl AsRef<std::path::Path>) -> Result<ParseContext, ParseError> {
	let slotitem_json_path = src.as_ref().join("kcwiki_slotitem.json");
	let useitem_json_path = src.as_ref().join("kcwiki_useitem.json");
	let ship_json_path = src.as_ref().join("kcwiki_ship.json");

	let slotitem_name_map = slot_item::parse_slotitem_name_mapping(&slotitem_json_path)?;
	let useitem_name_map = use_item::parse_useitem_name_mapping(&useitem_json_path)?;
	let ship_name_map = ship::parse_ship_name_mapping(&ship_json_path)?;

	Ok(ParseContext {
		slotitem_name_map,
		useitem_name_map,
		ship_name_map,
	})
}

pub fn parse(
	src: impl AsRef<std::path::Path>,
	_manifest: &ApiManifest,
) -> Result<Kc3rdSlotItemMap, ParseError> {
	let context = prepare_context(&src)?;

	let slot_item_parsed = {
		let json_path = src.as_ref().join("kcwiki_slotitem.json");
		slot_item::parse(&context, &json_path)?
	};

	let _ship_parsed = {
		let json_path = src.as_ref().join("kcwiki_ship.json");
		ship::parse(&context, &json_path)?
	};

	Ok(slot_item_parsed.map)
}

#[cfg(test)]
mod tests {
	use crate::parser::kcwiki::{ship, slot_item};
	use test_log::test;

	fn get_parse_context() -> super::ParseContext {
		let pwd = std::env::current_dir().unwrap();
		println!("current dir: {:?}", pwd);

		let src = std::path::Path::new("../../.data/temp");
		super::prepare_context(src).unwrap()
	}

	#[test]
	fn test_parse_slotitem() {
		let context = get_parse_context();
		let map = slot_item::parse(
			&context,
			std::path::Path::new("../../.data/temp/kcwiki_slotitem.json"),
		)
		.unwrap();

		let raw = serde_json::to_string_pretty(&map.map).unwrap();
		// save to file
		std::fs::write("../../.data/temp/kcwiki_slotitem_parsed.json", raw).unwrap();
		println!("slotitem: {}", map.map.len());
	}

	#[test]
	fn test_parse_ship() {
		let context = get_parse_context();
		let map = ship::parse(&context, std::path::Path::new("../../.data/temp/kcwiki_ship.json"))
			.unwrap();

		let raw = serde_json::to_string_pretty(&map).unwrap();
		// save to file
		std::fs::write("../../.data/temp/kcwiki_ship_parsed.json", raw).unwrap();
		println!("ship: {}", map.len());
	}
}
