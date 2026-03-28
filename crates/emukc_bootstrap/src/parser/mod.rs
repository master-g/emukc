//! Parsers for various data sources.

pub mod error;
pub mod kc3kai;
pub mod kcanotify;
pub mod kccp;
pub mod kcwiki;
pub mod kcwikizh_kcdata;
pub mod music;
pub mod tsunkit_nav;
pub mod tsunkit_quest;

use std::str::FromStr;

use emukc_model::{
	codex::{game_config::GameConfig, map::MapCatalog},
	kc2::navy::KcNavy,
	prelude::*,
};

use error::ParseError;
pub use kc3kai::parse as parse_kc3kai;
pub use kccp::quest::parse as parse_kccp_quests;
pub use kcwiki::parse as parse_kcwiki;
pub use kcwikizh_kcdata::parse as parse_kcdata;
pub use tsunkit_nav::parse as parse_tsunkit_nav;
pub use tsunkit_quest::parse as parse_tsunkit_quests;

fn load_map_catalog(
	dir: &std::path::Path,
	manifest: &ApiManifest,
) -> Result<MapCatalog, ParseError> {
	Ok(MapCatalog::load_baked_or_kcdata_root(dir.join("kc_data"), manifest))
}

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
		let raw =
			std::fs::read_to_string(&path).map_err(|source| ParseError::io_at(&path, source))?;
		debug!("Parsing manifest from {:?}", path);
		ApiManifest::from_str(&raw).map_err(|source| ParseError::json_at(&path, source))?
	};

	let (ship_extra, slotitem_extra_info) = parse_kcwiki(dir, &manifest)?;

	let (ship_picturebook, ship_class_name) = parse_kcdata(dir.join("kc_data"), &manifest)?;
	let kccp_quests = {
		let path = dir.join("kccp_quests.json");
		let raw =
			std::fs::read_to_string(&path).map_err(|source| ParseError::io_at(&path, source))?;
		debug!("Parsing kccp quests from {:?}", path);
		parse_kccp_quests(&raw).map_err(|source| {
			ParseError::Generic(format!("failed to parse {}: {source}", path.display()))
		})?
	};
	let quest = parse_tsunkit_quests(dir.join("tsunkit_quests.json"), &manifest, &kccp_quests)?;

	let expedition_conditions = {
		let path = dir.join("kcanotify_expedition.json");
		debug!("Parsing KCanotify expedition data from {:?}", path);
		kcanotify::expedition::parse(&path)?
	};

	let music_list = music::get()?;

	let mut cache_source = CacheSource::default();
	{
		let path = dir.join("kc3kai_jp_quotes.json");
		let raw =
			std::fs::read_to_string(&path).map_err(|source| ParseError::io_at(&path, source))?;
		let cleaned = raw
			.trim_start_matches('\u{FEFF}') // UTF-8 BOM
			.trim_start_matches('\u{FFFE}') // UTF-16 BOM
			.trim_start_matches(['\0', '\x01', '\x02', '\x03', '\x04', '\x05']) // controls
			.trim_start(); // whitespace
		parse_kc3kai(cleaned, &mut cache_source).map_err(|source| {
			ParseError::Generic(format!("failed to parse {}: {source}", path.display()))
		})?;
	}

	let maps = load_map_catalog(dir, &manifest)?;

	Ok(Codex {
		manifest,
		ship_extra,
		ship_class_name,
		ship_picturebook,
		slotitem_extra_info,
		quest,
		expedition_conditions,
		picturebook_extra: Kc3rdPicturebookExtra::default(),
		navy: KcNavy::default(),
		game_cfg: GameConfig::default(),
		music_list,
		maps,
		cache_source: Some(cache_source),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn load_map_catalog_uses_kcdata_when_baked_catalog_is_empty() {
		let root = tempfile::tempdir().unwrap();
		let kcdata_dir = root.path().join("kc_data/_map");
		std::fs::create_dir_all(&kcdata_dir).unwrap();

		std::fs::write(
			kcdata_dir.join("0012.yaml"),
			r#"data:
  id: 12
  name: "1-2 fallback"
  routes:
    1:
      to: 1
    2:
      from: 1
      to: 2
  cells:
    "1":
      name: "battle"
    "2":
      boss: true
"#,
		)
		.unwrap();

		let catalog = load_map_catalog(root.path(), &ApiManifest::default()).unwrap();
		let map_12 = catalog.map_definition(12).unwrap();

		assert_eq!(map_12.variant("").unwrap().boss_cell_no, 2);
	}
}
