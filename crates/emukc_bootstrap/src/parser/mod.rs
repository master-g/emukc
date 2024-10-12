//! Parsers for various data sources.

pub mod error;
pub mod kaisou;
pub mod kccp;
pub mod kcwiki_slotitems;
pub mod kcwikizh_kcdata;
pub mod kcwikizh_ships;
pub mod tsunkit_quest;

use std::str::FromStr;

use emukc_model::{kc2::navy::KcNavy, prelude::*, profile::material::MaterialConfig};

use error::ParseError;
pub use kaisou::parse as parse_kaisou;
pub use kccp::quest::parse as parse_kccp_quests;
pub use kcwiki_slotitems::parse as parse_kcwiki_slotitems;
pub use kcwikizh_kcdata::parse as parse_kcdata;
pub use kcwikizh_ships::parse as parse_ships_nedb;
pub use tsunkit_quest::parse as parse_tsunkit_quests;

/// Parse a partial codex from the given directory.
///
/// # Arguments
///
/// * `dir` - The directory to parse.
///
/// # Returns
///
/// A partial codex.
pub fn parse_partial_codex(dir: impl AsRef<std::path::Path>) -> Result<Codex, ParseError> {
	let dir = dir.as_ref();
	let manifest = {
		let path = dir.join("start2.json");
		let raw = std::fs::read_to_string(&path)?;
		ApiManifest::from_str(&raw)?
	};
	let ship_basic = parse_ships_nedb(dir.join("ships.nedb"))?;
	let slotitem_extra_info = parse_kcwiki_slotitems(dir.join("kcwiki_slotitem.json"))?;
	let (ship_extra_info, ship_class_name) = parse_kcdata(dir.join("kc_data"), &manifest)?;
	let ship_remodel_info = parse_kaisou(dir.join("main.js"), &manifest)?;
	let kccp_quests = {
		let path = dir.join("kccp_quests.json");
		let raw = std::fs::read_to_string(&path)?;
		parse_kccp_quests(&raw)?
	};
	let quest = parse_tsunkit_quests(dir.join("tsunkit_quests.json"), &manifest, &kccp_quests)?;

	Ok(Codex {
		manifest,
		ship_basic,
		ship_class_name,
		ship_extra_info,
		slotitem_extra_info,
		ship_remodel_info,
		quest,
		picturebook_extra: Kc3rdPicturebookExtra::default(),
		navy: KcNavy::default(),
		material_cfg: MaterialConfig::default(),
	})
}
