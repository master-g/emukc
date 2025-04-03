use anyhow::Result;
use clap::{Args, Subcommand};
use make_list::MakeListArguments;
use populate::PopulateArguments;

use crate::cfg::AppConfig;

mod foo;
mod make_list;
mod populate;

#[derive(Debug, Subcommand)]
enum Commands {
	#[command(about = "Generate cache list manifest")]
	MakeList(MakeListArguments),
	#[command(about = "Populate cache with list file")]
	Populate(PopulateArguments),
	#[command(about = "foo")]
	Foo,
}

#[derive(Debug, Args)]
pub(super) struct CacheArgs {
	#[command(subcommand)]
	command: Commands,
}

pub(super) async fn exec(args: &CacheArgs, config: &AppConfig) -> Result<()> {
	match &args.command {
		Commands::Populate(args) => populate::exec(args, config).await?,
		Commands::MakeList(args) => make_list::exec(args, config).await?,
		Commands::Foo => foo::exec(config).await?,
	}

	Ok(())
}
