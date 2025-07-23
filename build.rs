//! This script reads git commit hash and writes it to a file that will be included in the build.

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;

fn main() {
	let out_dir = match env::var("OUT_DIR") {
		Ok(val) => val,
		Err(e) => {
			eprintln!("error reading OUT_DIR: {e}");
			std::process::exit(1);
		}
	};

	let dest_path = Path::new(&out_dir).join("git_version.rs");

	let git_hash = get_git_commit_hash().unwrap_or_else(|| "unknown".to_string());

	let build_version = format!("pub const GIT_HASH: &str = \"{git_hash}\";");

	if let Err(e) = fs::write(&dest_path, build_version) {
		eprintln!("error writing to {}: {}", dest_path.display(), e);
		std::process::exit(1);
	}

	println!("cargo:rerun-if-changed=.git/HEAD");
	println!("cargo:rerun-if-changed=.git/refs/heads/");
}

fn get_git_commit_hash() -> Option<String> {
	// exec `git rev-parse --short HEAD`
	let output = Command::new("git")
		.args(["rev-parse", "--short", "HEAD"])
		.stdout(Stdio::piped())
		.output()
		.ok()?;

	if output.status.success() {
		let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
		Some(hash)
	} else {
		None
	}
}
