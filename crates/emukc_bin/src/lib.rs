//! emukc_bin is the executable crate for emukc.

#![deny(clippy::mem_forget)]
#![forbid(unsafe_code)]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

#[doc(hidden)]
pub mod cli;
