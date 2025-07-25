//! The `emukc_bootstrap` crate provides the bootstrap utilities for the `EmuKC` project.
//!
//! This crate handles:
//! - Downloading and parsing game data (ships, items, quests, etc.)
//! - Preparing the database with initial data
//! - Creating cache lists for game assets
//! - Populating the database with parsed data
//!
//! The bootstrap process ensures that all necessary game data and assets are available
//! for the `EmuKC` server to function properly.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

mod db;
mod download;
mod make_list;
mod parser;
mod populate;
mod res;

/// The `emukc_bootstrap` crate prelude.
pub mod prelude {
	pub use crate::db::{DbBootstrapError, prepare};
	pub use crate::download::BootstrapDownloadError;
	pub use crate::download::download_all;
	pub use crate::make_list::{
		CacheListMakeStrategy, errors::CacheListMakingError, make as make_cache_list,
	};
	pub use crate::parser::parse_partial_codex;
	pub use crate::populate::populate;
}
