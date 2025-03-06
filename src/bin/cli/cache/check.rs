use anyhow::Result;
use clap::Args;

use crate::{cfg::AppConfig, state::State};

#[derive(Debug, Args)]
pub(super) struct CheckArgs {
	#[arg(help = "Dry run, do not modify anything")]
	#[arg(long)]
	dry: bool,
}

pub(super) async fn exec(args: &CheckArgs, config: &AppConfig) -> Result<()> {
	let state = State::new(config).await?;
	state.kache.check_all(!args.dry).await?;

	Ok(())
}
