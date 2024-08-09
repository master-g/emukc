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

/// A type alias for `std::sync::Arc<Codex>`.
pub type CodexRaw = std::sync::Arc<Codex>;
