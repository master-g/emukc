//! Cosmetic seasons: the ships' art and voices as they were on a date.
//!
//! See `docs/plans/2026-10-15-007-feat-seasonal-archive-plan.md` (U5).

use anyhow::Result;
use clap::{Args, Subcommand};
use emukc_internal::prelude::*;

use crate::cfg::AppConfig;

#[derive(Args, Debug)]
pub(super) struct SeasonArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Fetch the shipgraph of every season in `seasons.json` that is not yet in the workspace.
    Sync,
}

pub(super) async fn exec(args: &SeasonArgs, cfg: &AppConfig) -> Result<()> {
    match args.command {
        Command::Sync => {
            for season in load_repo_seasons()? {
                let state =
                    if sync_season(&cfg.workspace_root, &season, cfg.proxy.as_deref()).await? {
                        "fetched"
                    } else {
                        "already present"
                    };
                println!(
                    "season {} ({}) {state} at {}",
                    season.id,
                    season.label,
                    season_shipgraph_path(&cfg.workspace_root, &season.id).display()
                );
            }
        }
    }
    Ok(())
}
