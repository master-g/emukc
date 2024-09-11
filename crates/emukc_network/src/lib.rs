//! The `emukc_network` crate provides the networking facilities for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

pub mod client;
pub mod download;

/// Re-export the `reqwest` crate.
pub use reqwest;

pub mod prelude {
	//! The `emukc_network` crate prelude.
	#[doc(hidden)]
	pub use crate::client::new_reqwest_client;
}
