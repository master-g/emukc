//! Emukc daemon.

#![deny(clippy::mem_forget)]
#![forbid(unsafe_code)]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

use std::process::ExitCode;

use emukc_internal::app::with_enough_stack;

mod cfg;
mod cli;
mod net;
mod state;

fn main() -> ExitCode {
	with_enough_stack(cli::init())
}
