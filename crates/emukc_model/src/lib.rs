//! The `emukc_model` crate provides the data model for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

pub mod cache;
pub mod codex;
pub mod fields;
pub mod kc2;
pub mod profile;
pub mod thirdparty;
pub mod user;

pub mod prelude {
	//! The `emukc_model` crate prelude.
	#[doc(hidden)]
	pub use crate::{
		cache::*, codex::Codex, fields::*, kc2::api::*, kc2::level::*, kc2::remodel::*,
		kc2::start2::*, kc2::types::*, thirdparty::*,
	};
}
