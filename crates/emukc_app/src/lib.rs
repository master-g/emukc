//! The `emukc_app` crate provides the basic app utilities for the `EmuKC` project.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod macros;
mod mem;

#[doc(hidden)]
pub mod cst;

#[doc(hidden)]
pub mod env;

/// Rust's default thread stack size of 2MiB doesn't allow sufficient recursion depth.
pub fn with_enough_stack<T>(fut: impl std::future::Future<Output = T> + Send) -> T {
	// Start a Tokio runtime with custom configuration
	tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.max_blocking_threads(*cst::RUNTIME_MAX_BLOCKING_THREADS)
		.thread_stack_size(*cst::RUNTIME_STACK_SIZE)
		.thread_name("emukc-worker")
		.build()
		.unwrap()
		.block_on(fut)
}

// Re-export
pub use tokio;

pub mod prelude {
	//! The `emukc_app` crate prelude.
	#[doc(hidden)]
	pub use crate::cst::{LOGO, PKG_VERSION, RUNTIME_MAX_BLOCKING_THREADS, RUNTIME_STACK_SIZE};

	#[doc(hidden)]
	pub use crate::env::{VERSION, arch, os};

	#[doc(hidden)]
	pub use crate::with_enough_stack;
}
