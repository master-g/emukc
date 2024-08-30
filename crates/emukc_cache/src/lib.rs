//! The `emukc_cache` crate provides the `KanColle` CDN file cache utilities for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

pub mod kache;

// TODO: add `export`, `import`, `repair` features

pub mod prelude {
	//! The `emukc_cache` crate prelude.
	//!
	#[doc(hidden)]
	pub use crate::kache::Kache;
}
