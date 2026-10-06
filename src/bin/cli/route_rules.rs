use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Subcommand};
use emukc_internal::prelude::*;

/// Maintenance commands for the routing rules converted from the compass simulator source.
#[derive(Args, Debug)]
pub(super) struct RouteRulesArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Fetch the pinned commit of the compass simulator source.
    Sync(SyncArgs),
}

#[derive(Args, Debug)]
struct SyncArgs {
    /// Directory the source is unpacked under, as `x20a_compass/<commit>/`.
    #[arg(long, default_value = ".data/temp", value_name = "DIR")]
    data_root: PathBuf,

    /// Proxy URL used for the download.
    #[arg(long, value_name = "URL")]
    proxy: Option<String>,
}

pub(super) async fn exec(args: &RouteRulesArgs) -> Result<()> {
    match &args.command {
        Command::Sync(sync) => {
            let (dir, downloaded) =
                sync_compass_source(&sync.data_root, sync.proxy.as_deref()).await?;
            let state = if downloaded {
                "fetched"
            } else {
                "already present"
            };
            println!("compass source {COMPASS_SOURCE_COMMIT} {state} at {}", dir.display());
        }
    }
    Ok(())
}
