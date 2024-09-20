use clap::{Args, Subcommand};

use crate::state::State;

mod check;
mod import;

#[derive(Debug, Subcommand)]
enum Commands {
	#[command(about = "Check cache files integrity")]
	Check(check::CheckArgs),
	#[command(about = "Import cached files from KCCP")]
	Import(import::ImportArgs),
}

#[derive(Debug, Args)]
pub(super) struct CacheArgs {
	#[command(subcommand)]
	command: Commands,
}

pub(super) async fn exec(
	args: &CacheArgs,
	state: &State,
) -> Result<(), Box<dyn std::error::Error>> {
	match &args.command {
		Commands::Check(args) => check::exec(&args, &state.kache).await?,
		Commands::Import(args) => import::exec(&args, &state.kache).await?,
	}

	Ok(())
}
