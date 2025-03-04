use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use emukc_internal::prelude::{Kache, import_kccp_cache};

#[derive(Debug, Args)]
pub(super) struct ImportArgs {
	#[arg(help = "Path to the KCCP cache json file")]
	#[arg(long)]
	json_path: PathBuf,

	#[arg(help = "Path to the cache root")]
	#[arg(long)]
	cache_root: Option<PathBuf>,
}

pub(super) async fn exec(args: &ImportArgs, kache: &Kache) -> Result<()> {
	import_kccp_cache(kache, &args.json_path, args.cache_root.as_deref()).await?;

	Ok(())
}
