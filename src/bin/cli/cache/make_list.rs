use anyhow::Result;
use clap::Args;

use crate::{cfg::AppConfig, state};

#[derive(Args, Debug)]
pub(super) struct MakeListArguments {
	#[arg(help = "Output file path")]
	#[arg(long)]
	pub output: Option<String>,

	#[arg(help = "Overwrite existing file")]
	#[arg(long)]
	pub overwrite: bool,
}

/// Make cache resources file list
pub(super) async fn exec(args: &MakeListArguments, config: &AppConfig) -> Result<()> {
	let state =
		state::State::new_with_custom_kache(config, |builder| builder.with_fast_check(true))
			.await?;

	let output = args.output.clone().unwrap_or_else(|| {
		config.cache_root.join("cache_resources.nedb").to_string_lossy().into_owned()
	});

	emukc_internal::bootstrap::prelude::make_cache_list(
		&state.codex.manifest,
		&state.kache,
		&output,
		args.overwrite,
	)
	.await?;

	Ok(())
}
