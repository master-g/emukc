//! Example: convert a normalized `MapCatalog` JSON file into a baked Rust module.

use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
	name = "tsunkit_map_codegen",
	about = "Deprecated wrapper around `tsunkit_map_data_downloader codegen`",
	long_about = "Use the merged tool instead:\n\n  cargo run --example tsunkit_map_data_downloader -- codegen --input .data/generated/tsunkit_map_catalog.json --output crates/emukc_model/src/codex/generated_map_catalog.rs"
)]
struct Args {
	/// Input JSON created by `tsunkit_map_data_downloader normalize` or `sync`.
	#[arg(long, default_value = ".data/generated/tsunkit_map_catalog.json", value_name = "FILE")]
	input: PathBuf,

	/// Output Rust module that will be checked into the repository.
	#[arg(
		long,
		default_value = "crates/emukc_model/src/codex/generated_map_catalog.rs",
		value_name = "FILE"
	)]
	output: PathBuf,
}

fn main() {
	let args = Args::parse();
	let status = std::process::Command::new("cargo")
		.args(["run", "--example", "tsunkit_map_data_downloader", "--", "codegen", "--input"])
		.arg(&args.input)
		.args(["--output"])
		.arg(&args.output)
		.status()
		.expect("failed to launch merged tsunkit map tool");
	if !status.success() {
		std::process::exit(status.code().unwrap_or(1));
	}
}
