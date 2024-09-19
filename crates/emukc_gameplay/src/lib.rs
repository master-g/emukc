//! The `emukc_gameplay` crate provides the game play implementation for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

#[doc(hidden)]
pub mod account;

pub mod gameplay;

pub mod prelude {
	//! The `emukc_gameplay` crate prelude.

	#[doc(hidden)]
	pub use crate::gameplay::Gameplay;
}