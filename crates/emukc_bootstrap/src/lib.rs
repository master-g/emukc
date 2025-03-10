//! The `emukc_bootstrap` crate provides the bootstrap utilities for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

mod cache;
mod crawl;
mod db;
mod download;
mod make_list;
mod parser;
mod res;

/// The `emukc_bootstrap` crate prelude.
pub mod prelude {
	pub use crate::cache::import_kccp_cache;
	pub use crate::crawl::crawl;
	pub use crate::db::{DbBootstrapError, prepare, prepare_cache};
	pub use crate::download::BootstrapDownloadError;
	pub use crate::download::download_all;
	pub use crate::make_list::{errors::CacheListMakingError, make as make_cache_list};
	pub use crate::parser::parse_partial_codex;
}
