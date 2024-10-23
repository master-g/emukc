//! Parsers for various data sources.

pub mod error;
pub mod kccp;
pub mod kcwiki;
pub mod kcwikizh_kcdata;
pub mod tsunkit_quest;

use std::str::FromStr;

use emukc_model::{kc2::navy::KcNavy, prelude::*, profile::material::MaterialConfig};

use error::ParseError;
pub use kccp::quest::parse as parse_kccp_quests;
pub use kcwiki::parse as parse_kcwiki;
pub use kcwikizh_kcdata::parse as parse_kcdata;
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

	let (ship_extra, slotitem_extra_info) = parse_kcwiki(dir, &manifest)?;

	let (ship_picturebook, ship_class_name) = parse_kcdata(dir.join("kc_data"), &manifest)?;
	let kccp_quests = {
		let path = dir.join("kccp_quests.json");
		let raw = std::fs::read_to_string(&path)?;
		parse_kccp_quests(&raw)?
	};
	let quest = parse_tsunkit_quests(dir.join("tsunkit_quests.json"), &manifest, &kccp_quests)?;

	Ok(Codex {
		manifest,
		ship_extra,
		ship_class_name,
		ship_picturebook,
		slotitem_extra_info,
		quest,
		picturebook_extra: Kc3rdPicturebookExtra::default(),
		navy: KcNavy::default(),
		material_cfg: MaterialConfig::default(),
	})
}
