use anyhow::Result;
use clap::Args;
use emukc::bootstrap::prelude::CacheListMakeStrategy;

use crate::{cfg::AppConfig, state};

#[derive(Args, Debug)]
pub(super) struct MakeListArguments {
    #[arg(help = "Output file path")]
    #[arg(long)]
    pub output: Option<String>,

    #[arg(help = "Overwrite existing file")]
    #[arg(long)]
    pub overwrite: bool,

    #[arg(help = "Use resource manifest instead of hardcoded lists")]
    #[arg(long)]
    pub manifest: bool,
}

/// Make cache resources file list
pub(super) async fn exec(args: &MakeListArguments, config: &AppConfig) -> Result<()> {
    let state = state::State::new(config, true).await?;

    let output = args.output.clone().unwrap_or_else(|| {
        config.cache_root.join("cache_resources.nedb").to_string_lossy().into_owned()
    });

    let strategy = if args.manifest {
        CacheListMakeStrategy::Manifest
    } else {
        CacheListMakeStrategy::Default
    };

    emukc_internal::bootstrap::prelude::make_cache_list(
        &state.codex,
        &state.kache,
        &output,
        strategy,
        args.overwrite,
    )
    .await?;

    Ok(())
}
