//! Emukc daemon.

use std::process::ExitCode;

use emukc_internal::app::with_enough_stack;

use emukc_bin::cli;

fn main() -> ExitCode {
	with_enough_stack(cli::init())
}
