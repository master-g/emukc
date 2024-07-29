//! The `emukc_model` crate provides the data model for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
extern crate tracing;

#[doc(hidden)]
mod account;
#[doc(hidden)]
mod kc2;
#[doc(hidden)]
mod thirdparty;
#[doc(hidden)]
mod user;

// Re-export the modules.

#[doc(inline)]
#[allow(unused_imports)]
pub use account::*;

#[doc(inline)]
#[allow(unused_imports)]
pub use kc2::*;

#[doc(inline)]
pub use thirdparty::*;

#[doc(inline)]
#[allow(unused_imports)]
pub(crate) use user::*;
