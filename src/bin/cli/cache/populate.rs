use anyhow::Result;
use clap::Args;

use crate::{cfg::AppConfig, state};

#[derive(Args, Debug)]
pub(super) struct PopulateArguments {
	#[arg(help = "Path to cache list file.")]
	#[arg(long)]
	pub src: Option<String>,

	#[arg(help = "skip checksum verification.")]
	#[arg(long)]
	pub skip_checksum: bool,
}

/// Populate cache with list file
pub(super) async fn exec(args: &PopulateArguments, config: &AppConfig) -> Result<()> {
	let state = state::State::new(config).await?;

	let src = args.src.clone().unwrap_or_else(|| {
		config.cache_root.join("cache_resources.nedb").to_string_lossy().into_owned()
	});

	emukc_internal::bootstrap::prelude::populate(&state.kache, &src, args.skip_checksum).await?;

	Ok(())
}
