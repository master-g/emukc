use std::{fs, path::Path, path::PathBuf, str::FromStr};

use anyhow::Result;
use clap::{Args, Subcommand};
use emukc_internal::prelude::*;

/// Maintenance commands for the repo-tracked map assets.
#[derive(Args, Debug)]
pub(super) struct MapArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build public map overlays from embedded real `api_req_map/start` captures.
    BuildOverlays(BuildOverlaysArgs),
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

pub(super) async fn exec(args: &MapArgs) -> Result<()> {
    match &args.command {
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

fn build_public_overlays(args: &BuildOverlaysArgs) -> Result<MapOverlayBuildOutput> {
    let manifest = read_manifest(&args.data_root)?;
    let (catalog, _report) =
        build_final_map_catalog(&args.data_root, &manifest).map_err(anyhow::Error::from)?;
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
