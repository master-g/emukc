use anyhow::Result;
use clap::Args;

use crate::{cfg::AppConfig, state};
use emukc_internal::prelude::crawl;

#[derive(Args, Debug)]
pub(super) struct CrawlArguments {
	#[arg(help = "Only check if file exists")]
	#[arg(long)]
	pub fast: bool,
}

/// Crawl from CDN
pub(super) async fn exec(args: &CrawlArguments, config: &AppConfig) -> Result<()> {
	let state =
		state::State::new_with_custom_kache(config, |builder| builder.with_fast_check(args.fast))
			.await?;
	crawl(&state.codex.manifest, &state.kache).await?;
	Ok(())
}
