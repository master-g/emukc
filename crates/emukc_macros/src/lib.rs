//! Macros for the emukc project

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// Matches on a specific config environment
#[allow(unused_macros)]
macro_rules! get_cfg {
	($i:ident : $($s:expr),+) => (
		let $i = || { $( if cfg!($i=$s) { return $s; } );+ "unknown"};
	)
}
