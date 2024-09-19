use emukc_internal::prelude::*;

/// Get the current release identifier
pub fn release() -> String {
	format!("{} for {} on {}", *PKG_VERSION, os(), arch())
}

/// Initialize the version command
pub async fn init() -> Result<(), ()> {
	// Initialize tracing and logging
	// Print local CLI version
	println!("{}", release());
	Ok(())
}
