use std::{collections::BTreeSet, fs, path::Path, path::PathBuf, str::FromStr};

use anyhow::Result;
use clap::{Args, Subcommand};
use emukc_internal::prelude::*;

/// Manual maintenance commands for the repo-tracked wikiwiki map catalog.
#[derive(Args, Debug)]
pub(super) struct WikiwikiMapArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Refresh local HTML cache from wikiwiki.jp.
    Sync(SyncArgs),
    /// Normalize agent-produced catalog JSON into the repo-tracked wikiwiki map catalog asset.
    Normalize(NormalizeArgs),
    /// Write a human-readable debug JSON file from agent-produced catalog JSON.
    Debug(DebugArgs),
    /// Build public map overlays from embedded real `api_req_map/start` captures.
    BuildOverlays(BuildOverlaysArgs),
}

#[derive(Args, Debug)]
struct NormalizeArgs {
    /// Directory containing `start2.json`.
    #[arg(long, default_value = ".data/temp", value_name = "DIR")]
    data_root: PathBuf,

    /// Path to the agent-produced `WikiwikiMapCatalog` JSON file.
    ///
    /// Generate this by running the `emukc-scrape-wikiwiki-mapdata` skill on
    /// cached HTML pages, then pass its output here.
    #[arg(long, value_name = "FILE")]
    from_agent_json: PathBuf,

    /// Output path for the normalized runtime `MapCatalog` JSON file.
    #[arg(long, default_value_os_t = repo_wikiwiki_map_catalog_path(), value_name = "FILE")]
    output: PathBuf,
}

#[derive(Args, Debug)]
struct SyncArgs {
    /// Directory containing `start2.json` and the `wikiwiki_map/` cache directory.
    #[arg(long, default_value = ".data/temp", value_name = "DIR")]
    data_root: PathBuf,

    /// Output path for the normalized runtime `MapCatalog` JSON file (unused by sync,
    /// kept for structural compatibility).
    #[arg(long, default_value_os_t = repo_wikiwiki_map_catalog_path(), value_name = "FILE")]
    output: PathBuf,

    /// Proxy URL used for wikiwiki requests.
    #[arg(long, value_name = "URL")]
    proxy: Option<String>,

    /// Overwrite existing files under `<data-root>/wikiwiki_map`.
    #[arg(long)]
    overwrite: bool,

    /// Limit sync to one or more map IDs, e.g. `1-1`.
    #[arg(long = "map", value_name = "MAP")]
    maps: Vec<String>,

    /// Maximum number of concurrent requests.
    #[arg(long, default_value_t = 2)]
    concurrent: usize,
}

#[derive(Args, Debug)]
struct DebugArgs {
    /// Path to the agent-produced `WikiwikiMapCatalog` JSON file.
    #[arg(long, value_name = "FILE")]
    from_agent_json: PathBuf,

    /// Output path for the human-readable debug JSON file.
    #[arg(
        long,
        default_value = ".data/generated/wikiwiki_map_catalog.debug.json",
        value_name = "FILE"
    )]
    output: PathBuf,
}

#[derive(Args, Debug)]
struct BuildOverlaysArgs {
    /// Directory containing `start2.json` and any supporting cache data used by catalog finalization.
    #[arg(long, default_value = ".data/temp", value_name = "DIR")]
    data_root: PathBuf,

    /// Output path for the normalized public overlay JSON file.
    #[arg(long, default_value_os_t = repo_public_map_catalog_overlay_path(), value_name = "FILE")]
    output: PathBuf,

    /// Output path for the overlay coverage report JSON file.
    #[arg(
        long,
        default_value = ".data/generated/public_map_catalog_overlays.report.json",
        value_name = "FILE"
    )]
    report_output: PathBuf,
}

pub(super) async fn exec(args: &WikiwikiMapArgs) -> Result<()> {
    match &args.command {
        Command::Sync(sync) => {
            let stats = run_sync(sync).await?;
            println!(
                "synced wikiwiki cache to {} (downloaded pages={}, failures={})",
                sync.data_root.join("wikiwiki_map").display(),
                stats.pages,
                stats.failures,
            );
        }
        Command::Normalize(paths) => {
            let catalog = normalize_catalog(paths)?;
            write_json(&catalog, &paths.output)?;
            println!("wrote {} wikiwiki maps to {}", catalog.maps.len(), paths.output.display());
        }
        Command::Debug(args) => {
            let raw = fs::read_to_string(&args.from_agent_json)?;
            let catalog = WikiwikiMapCatalog::from_json(&raw).map_err(anyhow::Error::from)?;
            write_json(&catalog.to_debug_json(), &args.output)?;
            println!("wrote {}", args.output.display());
        }
        Command::BuildOverlays(args) => {
            let output = build_public_overlays(args)?;
            write_json(&output.overlay, &args.output)?;
            write_json(&output.report, &args.report_output)?;
            println!(
                "{}",
                format_build_overlays_summary(&output, &args.output, &args.report_output),
            );
        }
    }

    Ok(())
}

fn read_manifest(data_root: &Path) -> Result<emukc::model::kc2::start2::ApiManifest> {
    let manifest_path = data_root.join("start2.json");
    let manifest_raw = fs::read_to_string(&manifest_path)?;
    Ok(emukc::model::kc2::start2::ApiManifest::from_str(&manifest_raw)?)
}

fn normalize_catalog(args: &NormalizeArgs) -> Result<emukc::model::codex::map::MapCatalog> {
    let manifest = read_manifest(&args.data_root)?;
    let raw = fs::read_to_string(&args.from_agent_json)?;
    let wikiwiki_source = WikiwikiMapCatalog::from_json(&raw).map_err(anyhow::Error::from)?;
    let wikiwiki_catalog = wikiwiki_source.into_map_catalog(&manifest);
    build_final_map_catalog(&args.data_root, &manifest, Some(wikiwiki_catalog))
        .map_err(anyhow::Error::from)
}

fn build_public_overlays(args: &BuildOverlaysArgs) -> Result<MapOverlayBuildOutput> {
    let manifest = read_manifest(&args.data_root)?;
    let catalog = build_final_map_catalog_from_repo_assets(&args.data_root, &manifest)
        .map_err(anyhow::Error::from)?;
    build_public_map_catalog_overlay_from_embedded_real_map_start_assets(
        &catalog,
        EMBEDDED_REAL_MAP_START_ASSETS,
    )
    .map_err(anyhow::Error::from)
}

fn format_build_overlays_summary(
    output: &MapOverlayBuildOutput,
    output_path: &Path,
    report_output_path: &Path,
) -> String {
    format!(
        "accepted {} overlay records from {} embedded sources; wrote {} and {}",
        output.report.accepted_records.len(),
        output.report.discovered_sources,
        output_path.display(),
        report_output_path.display(),
    )
}

fn write_json<T: serde::Serialize>(value: &T, output: &Path) -> Result<()> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

async fn run_sync(sync: &SyncArgs) -> Result<WikiwikiMapDownloadStats> {
    let map_filter = if sync.maps.is_empty() {
        None
    } else {
        Some(sync.maps.iter().cloned().collect::<BTreeSet<_>>())
    };

    download_wikiwiki_map_with_options(
        &sync.data_root,
        sync.overwrite,
        sync.proxy.as_deref(),
        WikiwikiMapDownloadOptions {
            concurrent: Some(sync.concurrent),
            map_filter,
            strict: true,
        },
    )
    .await
    .map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn build_overlay_summary_matches_report_semantics() {
        let output = MapOverlayBuildOutput {
            overlay: emukc::model::codex::map::MapCatalog::default(),
            report: MapOverlayBuildReport {
                discovered_sources: 35,
                accepted_records: vec![MapOverlayAcceptedRecord {
                    source: "map_1-1.json".to_string(),
                    map_id: 11,
                    stage_id: String::new(),
                    cell_count: 4,
                }],
                rejected_records: Vec::new(),
                known_map_count: 1,
                known_stage_count: 1,
                covered_map_count: 1,
                covered_stage_count: 1,
                uncovered_stages: Vec::new(),
            },
        };

        let summary = format_build_overlays_summary(
            &output,
            &PathBuf::from("overlay.json"),
            &PathBuf::from("overlay.report.json"),
        );

        assert_eq!(
            summary,
            "accepted 1 overlay records from 35 embedded sources; wrote overlay.json and overlay.report.json",
        );
    }
}
