use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Subcommand};
use emukc_internal::prelude::*;

/// Maintenance commands for the drops and enemy fleets taken from `KCNav`.
#[derive(Args, Debug)]
pub(super) struct KcnavArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Download the raw responses, one request at a time. Resumes where it stopped.
    Sync(SyncArgs),
    /// Turn the downloaded responses into one document, without touching the network.
    Normalize(NormalizeArgs),
}

#[derive(Args, Debug)]
struct SyncArgs {
    /// Directory the responses are stored under, as `kcnav/<map>/`.
    #[arg(long, default_value = ".data/temp", value_name = "DIR")]
    data_root: PathBuf,

    /// Codex directory; its map catalog says which edges end in a battle.
    #[arg(long, default_value = ".data/codex", value_name = "DIR")]
    codex: PathBuf,

    /// Limit the sync to one or more maps, e.g. `1-1`.
    #[arg(long = "map", value_name = "MAP")]
    maps: Vec<String>,

    /// Seconds to wait after every request; at least 1.
    #[arg(long, default_value_t = 2, value_name = "SECS")]
    interval: u64,

    /// Proxy URL used for the requests.
    #[arg(long, value_name = "URL")]
    proxy: Option<String>,

    /// Print how many requests the sync would make, and make none.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Args, Debug)]
struct NormalizeArgs {
    /// Directory written by `kcnav sync`.
    #[arg(long, default_value = ".data/temp/kcnav", value_name = "DIR")]
    input: PathBuf,

    /// Where to write the normalized document.
    #[arg(long, default_value = ".data/temp/kcnav.normalized.json", value_name = "FILE")]
    output: PathBuf,

    /// Codex directory; its map catalog says which variants have which nodes.
    #[arg(long, default_value = ".data/codex", value_name = "DIR")]
    codex: PathBuf,

    /// Where to write the ship drop asset.
    #[arg(long, default_value_os_t = repo_map_ship_drops_path(), value_name = "FILE")]
    ship_drops: PathBuf,
}

fn write_json<T: serde::Serialize>(value: &T, path: &std::path::Path) -> Result<()> {
    let mut json = serde_json::to_string_pretty(value)?;
    json.push('\n');
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))
}

pub(super) async fn exec(args: &KcnavArgs) -> Result<()> {
    match &args.command {
        Command::Sync(args) => {
            let codex = Codex::load_without_cache_source(&args.codex)
                .with_context(|| format!("loading the codex from {}", args.codex.display()))?;
            let edges = kcnav_battle_edges(&codex.map_catalog());
            if args.dry_run {
                let selected = edges
                    .iter()
                    .filter(|(map, _)| args.maps.is_empty() || args.maps.contains(map))
                    .collect::<Vec<_>>();
                let requests =
                    1 + selected.iter().map(|(_, edges)| 1 + 2 * edges.len()).sum::<usize>();
                println!(
                    "kcnav: {} maps, at most {requests} requests, about {} minutes at {}s apart",
                    selected.len(),
                    requests as u64 * args.interval.max(1) / 60,
                    args.interval.max(1)
                );
                return Ok(());
            }
            let options = KcnavSyncOptions {
                maps: args.maps.iter().cloned().collect(),
                interval_secs: args.interval,
            };
            let stats =
                sync_kcnav(&args.data_root, &edges, &options, args.proxy.as_deref()).await?;
            println!(
                "kcnav: {} downloaded, {} already present, {} failed under {}",
                stats.downloaded,
                stats.skipped,
                stats.failed,
                kcnav_dir(&args.data_root).display()
            );
            if stats.failed > 0 {
                bail!("{} requests failed; run the sync again to retry them", stats.failed);
            }
        }
        Command::Normalize(args) => {
            let catalog = normalize_kcnav(&args.input).map_err(|err| anyhow!(err))?;
            let nodes = catalog.maps.values().map(std::collections::BTreeMap::len).sum::<usize>();
            write_json(&catalog, &args.output)?;
            let codex = Codex::load_without_cache_source(&args.codex)
                .with_context(|| format!("loading the codex from {}", args.codex.display()))?;
            write_json(&kcnav_ship_drops(&catalog, &codex.map_catalog()), &args.ship_drops)?;
            println!(
                "wrote {nodes} nodes of {} maps to {}",
                catalog.maps.len(),
                args.output.display()
            );
        }
    }
    Ok(())
}
