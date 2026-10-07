use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use emukc_internal::prelude::*;
use serde::Deserialize;

/// Maintenance commands for the equipment bonus table converted from `KC3Kai`.
#[derive(Args, Debug)]
pub(super) struct GearBonusArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Fetch the pinned `KC3Kai` files the table is converted from.
    Sync(SyncArgs),
    /// Print the bonus of described ships and loadouts, without touching any database.
    Probe(ProbeArgs),
}

#[derive(Args, Debug)]
struct SyncArgs {
    /// Directory the files are kept under, as `kc3kai/<commit>/`.
    #[arg(long, default_value = ".data/temp", value_name = "DIR")]
    data_root: PathBuf,

    /// Proxy URL used for the download.
    #[arg(long, value_name = "URL")]
    proxy: Option<String>,
}

#[derive(Args, Debug)]
struct ProbeArgs {
    /// A JSON array of probes: a ship id and the equipment it carries.
    #[arg(long, value_name = "FILE")]
    input: PathBuf,

    /// Codex directory to read the table and the master data from.
    #[arg(long, default_value = ".data/codex", value_name = "DIR")]
    codex: PathBuf,
}

#[derive(Deserialize)]
struct Probe {
    ship: i64,
    /// Equipment id and improvement level.
    gears: Vec<(i64, i64)>,
}

pub(super) async fn exec(args: &GearBonusArgs) -> Result<()> {
    match &args.command {
        Command::Sync(sync) => {
            let (dir, downloaded) =
                sync_kc3kai_source(&sync.data_root, sync.proxy.as_deref()).await?;
            let state = if downloaded {
                "fetched"
            } else {
                "already present"
            };
            println!("KC3Kai source {KC3KAI_SOURCE_COMMIT} {state} at {}", dir.display());
        }
        Command::Probe(args) => {
            let codex = Codex::load_without_cache_source(&args.codex)
                .with_context(|| format!("loading the codex from {}", args.codex.display()))?;
            let raw = fs::read_to_string(&args.input)
                .with_context(|| format!("reading {}", args.input.display()))?;
            let bonuses = serde_json::from_str::<Vec<Probe>>(&raw)?
                .iter()
                .map(|probe| {
                    let ship = codex.find::<ApiMstShip>(&probe.ship)?;
                    let gears = probe
                        .gears
                        .iter()
                        .map(|(id, stars)| {
                            let mst = codex.find::<ApiMstSlotitem>(id)?;
                            Ok(GearBonusGear {
                                id: *id,
                                type2: mst.api_type[2],
                                type3: mst.api_type[3],
                                stars: *stars,
                                saku: mst.api_saku,
                                tyku: mst.api_tyku,
                                houm: mst.api_houm,
                            })
                        })
                        .collect::<Result<Vec<_>, CodexError>>()?;
                    codex.gear_bonus(ship, &gears)
                })
                .collect::<Result<Vec<_>, CodexError>>()?;
            println!("{}", serde_json::to_string(&bonuses)?);
        }
    }
    Ok(())
}
