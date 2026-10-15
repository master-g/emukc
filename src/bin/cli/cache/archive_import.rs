//! Import a snapshot of resources into the cache history, so each file answers for the version
//! it was taken at. The live cache and its recorded versions are left alone.
//!
//! See `docs/plans/2026-10-15-007-feat-seasonal-archive-plan.md` (U4).

use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};
use clap::Args;
use emukc_internal::{cache::IntoVersion, crypto::hash::sha256_hex};
use serde::Deserialize;

use crate::{cfg::AppConfig, state};

#[derive(Args, Debug)]
pub(super) struct ArchiveImportArguments {
    #[arg(help = "Snapshot directory: files under their resource paths, and a manifest.json")]
    pub dir: PathBuf,
}

#[derive(Deserialize)]
struct Manifest {
    files: Vec<ManifestFile>,
}

#[derive(Deserialize)]
struct ManifestFile {
    path: String,
    version: String,
    size: u64,
    sha256: String,
    last_modified: Option<String>,
}

pub(super) async fn exec(args: &ArchiveImportArguments, config: &AppConfig) -> Result<()> {
    let manifest: Manifest =
        serde_json::from_slice(&tokio::fs::read(args.dir.join("manifest.json")).await?)?;

    // Every file is checked before anything is kept: a snapshot that does not match its
    // manifest is not one to record history from.
    let mut kept = Vec::new();
    for file in &manifest.files {
        // "1" and "" mean no version here: nothing could ask for the file by its version.
        if file.version.as_str().into_version().is_none() {
            warn!("{} has no version ({:?}), left out", file.path, file.version);
            continue;
        }
        if Path::new(&file.path).components().any(|part| !matches!(part, Component::Normal(_))) {
            bail!("{} is not a plain resource path", file.path);
        }
        let data = tokio::fs::read(args.dir.join(&file.path)).await?;
        if data.len() as u64 != file.size || sha256_hex(&data) != file.sha256 {
            bail!("{} does not match its manifest size and sha256", file.path);
        }
        kept.push(file);
    }

    let state = state::State::new(config, false).await?;
    for file in &kept {
        let snapshot = args.dir.join(&file.path);
        state
            .kache
            .keep_version(&file.path, &file.version, &snapshot, file.last_modified.as_deref())
            .await?;
    }
    info!("kept {} files of {} in the cache history", kept.len(), args.dir.display());

    Ok(())
}
