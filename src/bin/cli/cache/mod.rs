use anyhow::Result;
use clap::{Args, Subcommand};

use crate::cfg::AppConfig;

mod check;
mod crawl;
mod import;

#[derive(Debug, Subcommand)]
enum Commands {
	#[command(about = "Check cache files integrity")]
	Check(check::CheckArgs),
	#[command(about = "Import cached files from KCCP")]
	Import(import::ImportArgs),
	#[command(about = "Crawl from CDN")]
	Crawl(crawl::CrawlArguments),
}

#[derive(Debug, Args)]
pub(super) struct CacheArgs {
	#[command(subcommand)]
	command: Commands,
}

pub(super) async fn exec(args: &CacheArgs, config: &AppConfig) -> Result<()> {
	match &args.command {
		Commands::Check(args) => check::exec(args, config).await?,
		Commands::Import(args) => import::exec(args, config).await?,
		Commands::Crawl(args) => crawl::exec(args, config).await?,
	}

	Ok(())
}
