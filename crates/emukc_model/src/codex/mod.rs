//! All the data need for running the game logic

use std::{fs::create_dir_all, str::FromStr};

use thiserror::Error;

use crate::{
	kc2,
	prelude::{Kc3rdPicturebookExtra, Kc3rdPicturebookRW},
	profile, thirdparty,
};

pub mod query;
pub mod ship;
pub mod slot_item;

/// Error type for `Codex`
#[derive(Error, Debug)]
pub enum CodexError {
	/// Entry already exists
	#[error("file {0} already exists")]
	AlreadyExist(String),

	/// IO error
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	/// Parse error
	#[error("Parse error: {0}")]
	Parse(#[from] std::num::ParseIntError),

	/// Serde error
	#[error("Serde error: {0}")]
	Serde(#[from] serde_json::Error),

	/// Entry not found
	#[error("Entry not found: {0}")]
	NotFound(String),
}

/// The `Codex` struct holds almost all the game data needed for the `EmuKC` project.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
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

	/// thirdparty picturebook extra info.
	pub picturebook_extra: thirdparty::Kc3rdPicturebookExtra,

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
const PATH_PICTUREBOOK_EXTRA_INFO: &str = "picturebook_extra_info.json";
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
	pub fn load(dir: impl AsRef<std::path::Path>) -> Result<Self, CodexError> {
		let path = dir.as_ref();

		let manifest = {
			let path = path.join(PATH_START2);
			let raw = std::fs::read_to_string(&path)?;
			kc2::start2::ApiManifest::from_str(&raw)?
		};

		let ship_basic = {
			let path = path.join(PATH_SHIP_BASIC);
			let raw = std::fs::read_to_string(&path)?;
			let data: Vec<thirdparty::Kc3rdShipBasic> = serde_json::from_str(&raw)?;
			data.into_iter().map(|v| (v.api_id, v)).collect()
		};

		let ship_class_name = {
			let path = path.join(PATH_SHIP_CLASS_NAME);
			let raw = std::fs::read_to_string(&path)?;
			let data: Vec<thirdparty::Kc3rdShipClassNameInfo> = serde_json::from_str(&raw)?;
			data.into_iter().map(|v| (v.api_id, v)).collect()
		};

		let ship_extra_info = {
			let path = path.join(PATH_SHIP_EXTRA_INFO);
			let raw = std::fs::read_to_string(&path)?;
			let data: Vec<thirdparty::Kc3rdShipExtraInfo> = serde_json::from_str(&raw)?;
			data.into_iter().map(|v| (v.api_id, v)).collect()
		};

		let slotitem_extra_info = {
			let path = path.join(PATH_SLOTITEM_EXTRA_INFO);
			let raw = std::fs::read_to_string(&path)?;
			let data: Vec<thirdparty::Kc3rdSlotItemExtraInfo> = serde_json::from_str(&raw)?;
			data.into_iter().map(|v| (v.api_id, v)).collect()
		};

		let ship_remodel_info = {
			let path = path.join(PATH_SHIP_REMODEL_INFO);
			let raw = std::fs::read_to_string(&path)?;
			let data: Vec<kc2::remodel::KcShipRemodelRequirement> = serde_json::from_str(&raw)?;
			data.into_iter().map(|v| ((v.id_from, v.id_to), v)).collect()
		};

		let picturebook_extra = {
			let path = path.join(PATH_PICTUREBOOK_EXTRA_INFO);
			let raw = std::fs::read_to_string(&path)?;
			let data: Kc3rdPicturebookRW = serde_json::from_str(&raw)?;
			let data: Kc3rdPicturebookExtra = data.into();
			data
		};

		let quest = {
			let path = path.join(PATH_QUEST);
			let raw = std::fs::read_to_string(&path)?;
			let data: Vec<thirdparty::Kc3rdQuest> = serde_json::from_str(&raw)?;
			data.into_iter().map(|v| (v.api_no, v)).collect()
		};

		Ok(Codex {
			manifest,
			ship_basic,
			ship_class_name,
			ship_extra_info,
			slotitem_extra_info,
			ship_remodel_info,
			picturebook_extra,
			navy: Self::load_single_item(path.join(PATH_NAVY))?,
			quest,
			material_cfg: Self::load_single_item(path.join(PATH_MATERIAL_CFG))?,
		})
	}

	/// Save `Codex` instance to directory.
	///
	/// # Arguments
	///
	/// * `dst` - The directory path.
	/// * `overwrite` - Whether to overwrite the existing files.
	///
	/// # Returns
	///
	/// Ok if success, otherwise an error.
	pub fn save(
		&self,
		dst: impl AsRef<std::path::Path>,
		overwrite: bool,
	) -> Result<(), CodexError> {
		let dst = dst.as_ref();
		if !dst.exists() {
			create_dir_all(dst)?;
		}

		// manifest
		{
			let path = dst.join(PATH_START2);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			std::fs::write(path, serde_json::to_string_pretty(&self.manifest)?)?;
		}
		// ship basic
		{
			let path = dst.join(PATH_SHIP_BASIC);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			let data = self.ship_basic.values().collect::<Vec<_>>();
			std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
		}
		// ship class name
		{
			let path = dst.join(PATH_SHIP_CLASS_NAME);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			let data = self.ship_class_name.values().collect::<Vec<_>>();
			std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
		}
		// ship extra info
		{
			let path = dst.join(PATH_SHIP_EXTRA_INFO);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			let data = self.ship_extra_info.values().collect::<Vec<_>>();
			std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
		}
		// slotitem extra info
		{
			let path = dst.join(PATH_SLOTITEM_EXTRA_INFO);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			let data = self.slotitem_extra_info.values().collect::<Vec<_>>();
			std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
		}
		// ship remodel info
		{
			let path = dst.join(PATH_SHIP_REMODEL_INFO);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			let data = self.ship_remodel_info.values().collect::<Vec<_>>();
			std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
		}
		// picturebook extra info
		{
			let path = dst.join(PATH_PICTUREBOOK_EXTRA_INFO);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			let data: Kc3rdPicturebookRW = self.picturebook_extra.clone().into();
			std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
		}
		// navy
		{
			let path = dst.join(PATH_NAVY);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			std::fs::write(path, serde_json::to_string_pretty(&self.navy)?)?;
		}
		// quest
		{
			let path = dst.join(PATH_QUEST);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			let data = self.quest.values().collect::<Vec<_>>();
			std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
		}
		// material cfg
		{
			let path = dst.join(PATH_MATERIAL_CFG);
			if path.exists() && !overwrite {
				return Err(CodexError::AlreadyExist(path.display().to_string()));
			}
			std::fs::write(path, serde_json::to_string_pretty(&self.material_cfg)?)?;
		}

		Ok(())
	}

	fn load_single_item<T>(path: impl AsRef<std::path::Path>) -> Result<T, CodexError>
	where
		T: serde::de::DeserializeOwned,
	{
		let path = path.as_ref();
		let raw = std::fs::read_to_string(path)?;

		Ok(serde_json::from_str(&raw)?)
	}
}
