//! Cosmetic seasons: the ships' art and voices as they were on a date.
//!
//! A season is a shipgraph taken from kcwiki's start2 of that date. Served in place of today's,
//! it makes the client ask for the versions of that date, which the cache answers from its
//! history. See `docs/plans/2026-10-15-007-feat-seasonal-archive-plan.md` (L1).

use std::path::{Path, PathBuf};

use emukc_network::{client::new_reqwest_client, download::Request};
use serde::Deserialize;

use crate::{download::BootstrapDownloadError, parser::error::ParseError};

/// One entry of `seasons.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Season {
    /// The date, such as `2026-10-14`.
    pub id: String,
    /// What the date is known for; for people only.
    pub label: String,
    /// The `kcwiki/kancolle-data` commit holding that date's start2.
    pub start2_commit: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SeasonsAsset {
    #[expect(dead_code, reason = "for the reader of the file")]
    note: String,
    seasons: Vec<Season>,
}

/// The seasons listed in the repository.
pub fn load_repo_seasons() -> Result<Vec<Season>, ParseError> {
    let asset = crate::assets::SEASONS;
    let path = asset.path();
    let (_, raw) = asset.load().map_err(|source| ParseError::io_at(&path, source))?;
    let asset: SeasonsAsset =
        serde_json::from_str(&raw).map_err(|source| ParseError::json_at(&path, source))?;
    Ok(asset.seasons)
}

/// Where the shipgraph of season `id` is kept under the workspace.
pub fn season_shipgraph_path(workspace_root: impl AsRef<Path>, id: &str) -> PathBuf {
    workspace_root.as_ref().join("seasons").join(format!("{id}.json"))
}

/// Fetch the start2 of `season` and keep its shipgraph at [`season_shipgraph_path`], unless it
/// is already there. Returns whether anything was downloaded.
#[expect(clippy::result_large_err)]
pub async fn sync_season(
    workspace_root: impl AsRef<Path>,
    season: &Season,
    proxy: Option<&str>,
) -> Result<bool, BootstrapDownloadError> {
    let target = season_shipgraph_path(&workspace_root, &season.id);
    if target.exists() {
        return Ok(false);
    }
    let start2 = target.with_extension("start2.json");
    if let Some(dir) = start2.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let client = new_reqwest_client(proxy, None).map_err(|source| {
        BootstrapDownloadError::ReqwestClient {
            proxy: proxy.map(ToOwned::to_owned),
            source,
        }
    })?;
    Request::builder()
        .url(format!(
            "https://raw.githubusercontent.com/kcwiki/kancolle-data/{}/api/api_start2.json",
            season.start2_commit
        ))
        .save_as(&start2)
        .overwrite(true)
        .skip_header_check(true)
        .build()?
        .execute(Some(client))
        .await?;

    let raw: serde_json::Value = serde_json::from_slice(&std::fs::read(&start2)?)
        .map_err(|e| BootstrapDownloadError::Generic(format!("{}: {e}", start2.display())))?;
    let Some(shipgraph) = raw.get("api_mst_shipgraph") else {
        return Err(BootstrapDownloadError::Generic(format!(
            "{} has no api_mst_shipgraph",
            start2.display()
        )));
    };
    // Written aside and moved in: a file cut short would otherwise be skipped as present.
    let partial = target.with_extension("json.partial");
    std::fs::write(&partial, serde_json::to_vec(shipgraph).map_err(std::io::Error::other)?)?;
    std::fs::rename(&partial, &target)?;
    std::fs::remove_file(&start2)?;
    Ok(true)
}
