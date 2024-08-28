//! The `emukc_bootstrap` crate provides the bootstrap utilities for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

#[doc(hidden)]
pub mod download;

pub(crate) mod parser;
pub(crate) mod res;

pub mod prelude {
	//! The `emukc_bootstrap` crate prelude.
	#[doc(hidden)]
	pub use crate::download::download_all;

	#[doc(hidden)]
	pub use crate::parser::{
		parse_kaisou, parse_kccp_quests, parse_kcdata, parse_ships_nedb, parse_tsunkit_quests,
	};
}
