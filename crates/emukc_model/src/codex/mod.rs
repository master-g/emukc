//! All the data need for running the game logic

use std::str::FromStr;

use crate::{kc2, profile, thirdparty};

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

const PATH_START2: &str = "start2.json";
const PATH_SHIP_BASIC: &str = "ship_basic.json";
const PATH_SHIP_CLASS_NAME: &str = "ship_class_name.json";
const PATH_SHIP_EXTRA_INFO: &str = "ship_extra_info.json";
const PATH_SLOTITEM_EXTRA_INFO: &str = "slotitem_extra_info.json";
const PATH_SHIP_REMODEL_INFO: &str = "ship_remodel_info.json";
const PATH_SHIP_EXTRA_VOICE: &str = "ship_extra_voice.json";
const PATH_NAVY: &str = "navy.json";
const PATH_QUEST: &str = "quest.json";
const PATH_MATERIAL_CFG: &str = "material_cfg.json";

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

		let manifest = {
			let path = path.join(PATH_START2);
			let raw = std::fs::read_to_string(&path)?;
			kc2::start2::ApiManifest::from_str(&raw)?
		};

		Ok(Codex {
			manifest,
			ship_basic: Self::load_single_item(path.join(PATH_SHIP_BASIC))?,
			ship_class_name: Self::load_single_item(path.join(PATH_SHIP_CLASS_NAME))?,
			ship_extra_info: Self::load_single_item(path.join(PATH_SHIP_EXTRA_INFO))?,
			slotitem_extra_info: Self::load_single_item(path.join(PATH_SLOTITEM_EXTRA_INFO))?,
			ship_remodel_info: Self::load_single_item(path.join(PATH_SHIP_REMODEL_INFO))?,
			ship_extra_voice: Self::load_single_item(path.join(PATH_SHIP_EXTRA_VOICE))?,
			navy: Self::load_single_item(path.join(PATH_NAVY))?,
			quest: Self::load_single_item(path.join(PATH_QUEST))?,
			material_cfg: Self::load_single_item(path.join(PATH_MATERIAL_CFG))?,
		})
	}

	fn load_single_item<T>(
		path: impl AsRef<std::path::Path>,
	) -> Result<T, Box<dyn std::error::Error>>
	where
		T: serde::de::DeserializeOwned,
	{
		let path = path.as_ref();
		let raw = std::fs::read_to_string(path)?;

		Ok(serde_json::from_str(&raw)?)
	}
}

/// A type alias for `std::sync::Arc<Codex>`.
pub type CodexRaw = std::sync::Arc<Codex>;
