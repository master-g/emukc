//! `KCWiki` data parser

use emukc_model::prelude::{ApiManifest, Kc3rdSlotItemExtraInfoMap};

use super::error::ParseError;

mod db_slotitem;
mod ship;
mod slotitem;

pub fn parse(
	src: impl AsRef<std::path::Path>,
	_manifest: &ApiManifest,
) -> Result<Kc3rdSlotItemExtraInfoMap, ParseError> {
	let db_slotitems_path = src.as_ref().join("kcwiki_db_slotitem.json");

	db_slotitem::parse(&db_slotitems_path)
}
