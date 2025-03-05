//! The `emukc_bootstrap` crate provides the bootstrap utilities for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

#[doc(hidden)]
pub mod download;

pub(crate) mod cache;
pub(crate) mod crawl;
pub(crate) mod db;
pub(crate) mod parser;
pub(crate) mod res;

pub mod prelude {
	//! The `emukc_bootstrap` crate prelude.

	#[doc(hidden)]
	pub use crate::cache::import_kccp_cache;

	#[doc(hidden)]
	pub use crate::crawl::crawl;

	#[doc(hidden)]
	pub use crate::download::download_all;

	#[doc(hidden)]
	pub use crate::db::{DbBootstrapError, prepare, prepare_cache};

	#[doc(hidden)]
	pub use crate::parser::parse_partial_codex;
}
