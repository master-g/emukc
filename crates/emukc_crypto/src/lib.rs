//! This library provides cryptographic functions and types.
//! Note that this module is not intended to be used directly by the user.
//! The cryptographic functions here are very simple and not secure.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(hidden)]
pub mod hash;
#[doc(hidden)]
pub mod password;

#[doc(inline)]
pub use hash::md5;
#[doc(inline)]
pub use hash::md5_file;
#[doc(inline)]
pub use hash::SimpleHash;
#[doc(inline)]
pub use password::PasswordCrypto;
