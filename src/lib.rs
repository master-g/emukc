#![allow(clippy::single_component_path_imports)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]

//! [![EmuKc Logo](http://203.104.209.71/kcs2/resources/useitem/card_/090.png)](https://github.com/master-g/emukc)
//!
//! `EmuKc` is a project that aims to provide a complete simulation of the game "Kantai Collection" (`KanColle`).
//!

pub use emukc_internal::*;

#[cfg(all(feature = "dynamic_linking", not(target_family = "wasm")))]
#[allow(unused_imports)]
use emukc_dylib;
