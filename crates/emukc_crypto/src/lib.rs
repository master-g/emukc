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
#[doc(hidden)]
pub mod suffix_utils;

#[doc(inline)]
pub use hash::md5;
#[doc(inline)]
pub use hash::md5_file;

#[cfg(feature = "async")]
#[doc(inline)]
pub use hash::md5_file_async;

#[doc(inline)]
pub use hash::SimpleHash;
#[doc(inline)]
pub use password::PasswordCrypto;

#[doc(inline)]
pub use suffix_utils::SuffixUtils;

pub mod prelude {
	//! The `emukc_crypto` crate prelude.
	#[doc(hidden)]
	pub use crate::{PasswordCrypto, SimpleHash, md5, md5_file};

	#[cfg(feature = "async")]
	#[doc(hidden)]
	pub use crate::md5_file_async;

	#[doc(hidden)]
	pub use crate::SuffixUtils;
}
