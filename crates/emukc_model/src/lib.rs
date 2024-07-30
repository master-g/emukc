//! The `emukc_model` crate provides the data model for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

#[doc(hidden)]
mod kc2;
#[doc(hidden)]
mod thirdparty;
#[doc(hidden)]
mod user;

// Re-export the modules.

#[doc(inline)]
#[allow(unused_imports)]
pub use kc2::*;

#[doc(inline)]
pub use thirdparty::*;

#[doc(inline)]
#[allow(unused_imports)]
pub(crate) use user::*;

/// The `Codex` struct holds almost all the game data needed for the `EmuKC` project.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Codex {
	/// KC2 API manifest.
	pub manifest: start2::ApiManifest,

	/// thirdparty ship basic info map.
	pub ship_basic: Kc3rdShipBasicMap,

	/// thirdparty ship class name map.
	pub ship_class_name: Kc3rdShipClassNameMap,

	/// thirdparty ship extra info map.
	pub ship_extra_info: Kc3rdShipExtraInfoMap,

	/// thirdparty slot item extra info map.
	pub slotitem_extra_info: Kc3rdSlotItemExtraInfoMap,

	/// thirdparty quest info map.
	pub quest: Kc3rdQuestMap,
}

/// A type alias for `std::sync::Arc<Codex>`.
pub type CodexRaw = std::sync::Arc<Codex>;
