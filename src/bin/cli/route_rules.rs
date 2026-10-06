use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
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
    /// Turn the decoder's neutral rule document into the repo-tracked routing-rule asset.
    Normalize(NormalizeArgs),
    /// Print where described fleets would go, as node label to probability, without rolling.
    Dist(DistArgs),
}

#[derive(Args, Debug)]
struct DistArgs {
    /// A JSON array of probes: map, variant, node, visited labels and the fleet.
    #[arg(long, value_name = "FILE")]
    input: PathBuf,

    /// Codex directory to read the map catalog from.
    #[arg(long, default_value = ".data/codex", value_name = "DIR")]
    codex: PathBuf,
}

#[derive(Args, Debug)]
struct NormalizeArgs {
    /// The document written by `bun run route-rules` in `main-decoder`. Defaults to the one
    /// next to the pinned source under `.data/temp`.
    #[arg(long, value_name = "FILE")]
    input: Option<PathBuf>,

    /// Where to write the asset.
    #[arg(long, default_value_os_t = repo_compass_route_rules_path(), value_name = "FILE")]
    output: PathBuf,
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
        Command::Dist(args) => {
            let codex = Codex::load_without_cache_source(&args.codex)
                .with_context(|| format!("loading the codex from {}", args.codex.display()))?;
            let raw = fs::read_to_string(&args.input)
                .with_context(|| format!("reading {}", args.input.display()))?;
            let probes = serde_json::from_str::<Vec<RouteProbe>>(&raw)?;
            let distributions = probes
                .iter()
                .map(|probe| probe_route(&codex, probe))
                .collect::<std::result::Result<Vec<_>, _>>()?;
            println!("{}", serde_json::to_string(&distributions)?);
        }
        Command::Normalize(args) => {
            let input = args.input.clone().unwrap_or_else(|| {
                compass_source_dir(".data/temp").with_extension("route_rules.json")
            });
            let raw = fs::read_to_string(&input)
                .with_context(|| format!("reading {}", input.display()))?;
            let asset = normalize_compass_route_rules(&raw).map_err(|err| anyhow!(err))?;
            let rules = asset
                .maps
                .values()
                .flat_map(|variants| variants.values())
                .map(|variant| variant.start.len() + variant.rules.len())
                .sum::<usize>();
            let mut json = serde_json::to_string_pretty(&asset)?;
            json.push('\n');
            fs::write(&args.output, json)
                .with_context(|| format!("writing {}", args.output.display()))?;
            println!(
                "wrote {rules} rules for {} maps to {}",
                asset.maps.len(),
                args.output.display()
            );
        }
    }
    Ok(())
}
