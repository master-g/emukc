use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use emukc_internal::prelude::import_kccp_cache;

use crate::{cfg::AppConfig, state::State};

#[derive(Debug, Args)]
pub(super) struct ImportArgs {
	#[arg(help = "Path to the KCCP cache json file")]
	#[arg(long)]
	json_path: PathBuf,

	#[arg(help = "Path to the cache root")]
	#[arg(long)]
	cache_root: Option<PathBuf>,
}

pub(super) async fn exec(args: &ImportArgs, config: &AppConfig) -> Result<()> {
	let state = State::new(config).await?;
	import_kccp_cache(&state.kache, &args.json_path, args.cache_root.as_deref()).await?;

	Ok(())
}
