//! The `emukc_cache` crate provides the `KanColle` CDN file cache utilities for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

mod error;
mod export;
mod kache;
mod opt;
mod ver;

pub use error::Error as KacheError;
pub use kache::Builder as KacheBuilder;
pub use kache::Kache;
pub use opt::GetOption;
pub use ver::{IntoVersion, NoVersion};

/// Convert a path to a unified relative path.
///
/// This function replaces backslashes with forward slashes and
/// trims leading slashes from the path.
pub fn unified_rel_path(path: &str) -> String {
	// unify the path to be relative
	path.replace('\\', "/").trim_start_matches('/').to_owned()
}

/// The `emukc_cache` crate prelude.
///
/// This module re-exports the core types and traits of the crate
/// for convenient importing with a global import: `use emukc_cache::prelude::*;`
pub mod prelude {
	pub use crate::GetOption;
	pub use crate::IntoVersion;
	pub use crate::Kache;
	pub use crate::KacheBuilder;
	pub use crate::KacheError;
	pub use crate::NoVersion;
}
