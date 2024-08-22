//! The `emukc_model` crate provides the data model for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

pub mod kc2;
pub mod profile;
pub mod thirdparty;
pub mod user;

/// The `Codex` struct holds almost all the game data needed for the `EmuKC` project.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Codex {
	/// KC2 API manifest.
	pub manifest: kc2::start2::ApiManifest,

	/// thirdparty ship basic info map.
	pub ship_basic: thirdparty::Kc3rdShipBasicMap,

	/// thirdparty ship class name map.
	pub ship_class_name: thirdparty::Kc3rdShipClassNameMap,

	/// thirdparty ship extra info map.
	pub ship_extra_info: thirdparty::Kc3rdShipExtraInfoMap,

	/// thirdparty slot item extra info map.
	pub slotitem_extra_info: thirdparty::Kc3rdSlotItemExtraInfoMap,

	/// ship remodel info map.
	pub ship_remodel_info: kc2::remodel::KcShipRemodelRequirementMap,

	/// thirdparty ship extrace voice info map.
	pub ship_extra_voice: thirdparty::Kc3rdShipVoiceMap,

	/// navy info.
	pub navy: kc2::navy::KcNavy,

	/// thirdparty quest info map.
	pub quest: thirdparty::Kc3rdQuestMap,

	/// Material config
	pub material_cfg: profile::material::MaterialConfig,
	// TODO: add more limitations.
}

impl Codex {
	/// Load `Codex` instance from directory.
	///
	/// the `ApiManifest` is loaded from `dir/start2.json`.
	///
	/// the `Kc3rdShipBasicMap` is loaded from `dir/ship_basic.json`.
	///
	/// the `Kc3rdShipClassNameMap` is loaded from `dir/ship_class_name.json`.
	///
	/// the `Kc3rdShipExtraInfoMap` is loaded from `dir/ship_extra_info.json`.
	///
	/// the `Kc3rdSlotItemExtraInfoMap` is loaded from `dir/slotitem_extra_info.json`.
	///
	/// the `KcShipRemodelRequirementMap` is loaded from `dir/ship_remodel_info.json`.
	///
	/// the `Kc3rdShipVoiceMap` is loaded from `dir/ship_extra_voice.json`.
	///
	/// the `KcNavy` is loaded from `dir/navy.json`.
	///
	/// the `Kc3rdQuestMap` is loaded from `dir/quest.json`.
	///
	/// the `MaterialConfig` is loaded from `dir/material_cfg.json`.
	///
	/// # Arguments
	///
	/// * `dir` - The directory path.
	///
	/// # Returns
	///
	/// The `Codex` instance.
	pub fn load(dir: impl AsRef<std::path::Path>) -> Result<Self, Box<dyn std::error::Error>> {
		let path = dir.as_ref();
		let file = std::fs::File::open(path)?;
		let reader = std::io::BufReader::new(file);
		let codex = serde_json::from_reader(reader)?;
		Ok(codex)
	}
}

/// A type alias for `std::sync::Arc<Codex>`.
pub type CodexRaw = std::sync::Arc<Codex>;

pub mod prelude {
	//! The `emukc_model` crate prelude.
	#[doc(hidden)]
	pub use crate::{kc2::start2::*, Codex, CodexRaw};
}
