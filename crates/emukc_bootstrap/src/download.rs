//! Download the resources

use emukc_crypto::md5_file;
use emukc_network::{client::new_reqwest_client, download::DownloadError, reqwest};
use futures::{StreamExt, TryStreamExt, stream::FuturesUnordered};
use std::sync::Arc;
use thiserror::Error;

use crate::progress::{
    download_aggregate_style, log_with_mp, new_multi_progress, new_progress_bar,
    new_progress_bar_on_mp, new_spinner_on_mp,
};
use crate::res::RES_LIST;

/// Error that can occur during the download process
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum BootstrapDownloadError {
    /// IO error
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Request builder error
    #[error(transparent)]
    Builder(#[from] emukc_network::download::BuilderError),

    /// Reqwest client error
    #[error("failed to build bootstrap reqwest client with proxy {proxy:?}: {source}")]
    ReqwestClient {
        proxy: Option<String>,
        #[source]
        source: reqwest::Error,
    },

    /// Download error
    #[error(transparent)]
    Download(#[from] emukc_network::download::DownloadError),

    /// All configured CDN sources failed for a required web asset.
    #[error("all CDN sources failed for web asset {path}")]
    WebAssetUnavailable {
        path: String,
    },

    /// Unzip error
    #[error("bootstrap resource {save_as} ({url}) failed while {action}: {source}")]
    Unzip {
        url: String,
        save_as: String,
        action: &'static str,
        #[source]
        source: zip::result::ZipError,
    },

    /// Resource IO error
    #[error(
		"bootstrap resource {save_as} ({url}) failed while {action} at {}: {source}",
		path.display()
	)]
    ResourceIo {
        url: String,
        save_as: String,
        action: &'static str,
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Resource JSON error
    #[error("bootstrap resource at {} failed while parsing JSON: {source}", path.display())]
    Json {
        path: std::path::PathBuf,
        #[source]
        source: serde_json::Error,
    },

    /// Resource timeout
    #[error("bootstrap resource {save_as} ({url}) timed out after {timeout_secs}s while {action}")]
    Timeout {
        url: String,
        save_as: String,
        action: &'static str,
        timeout_secs: u64,
    },

    /// Generic bootstrap failure
    #[error("{0}")]
    Generic(String),
}

/// Download all the resources
///
/// # Arguments
///
/// * `dir` - The output directory
/// * `overwrite` - Whether to overwrite existing files
/// * `proxy` - The proxy server
/// * `concurrent` - The maximum number of concurrent downloads, default is 4
pub async fn download_all(
    dir: impl AsRef<std::path::Path>,
    overwrite: bool,
    proxy: Option<&str>,
    concurrent: Option<usize>,
) -> Result<(), BootstrapDownloadError> {
    let output_dir = dir.as_ref();
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    debug!("proxy: {:?}", proxy);

    let client = Arc::new(new_reqwest_client(proxy, None).map_err(|source| {
        BootstrapDownloadError::ReqwestClient {
            proxy: proxy.map(ToOwned::to_owned),
            source,
        }
    })?);
    let max_concurrent = concurrent.unwrap_or(4).max(1);
    let mut tasks = FuturesUnordered::new();

    let mp = Arc::new(new_multi_progress());
    let aggregate_pb = match mp.as_ref() {
        Some(mp) => {
            let pb =
                new_progress_bar_on_mp(0, "Downloading resources", download_aggregate_style(), mp);
            Some(pb)
        }
        None => new_progress_bar(0, "Downloading resources", download_aggregate_style()),
    };

    // Count total resources to download
    let total_resources = RES_LIST
        .iter()
        .filter(|res| {
            let fullpath = output_dir.join(res.save_as);
            !fullpath.exists() || overwrite
        })
        .count();

    if let Some(ref pb) = aggregate_pb {
        pb.set_length(total_resources as u64);
    }

    for res in RES_LIST.iter() {
        let client = client.clone();
        let fullpath = output_dir.join(res.save_as);

        if fullpath.exists() && !overwrite {
            debug!("Skipping {:?} as it already exists", res);
            continue;
        }

        let output_dir = output_dir.to_path_buf();
        let aggregate_pb = aggregate_pb.clone();
        let mp = mp.clone();
        let spinner = mp.as_ref().as_ref().map(|mp| new_spinner_on_mp(res.save_as, mp));

        let task = async move {
            let since = std::time::Instant::now();
            let fullpath = output_dir.join(res.save_as);
            trace!("downloading {} from {}", res.save_as, res.url);

            let result = emukc_network::download::Request::builder()
                .url(res.url)
                .save_as(&fullpath)
                .overwrite(overwrite)
                .skip_header_check(true)
                .build()?
                .execute(Some((*client).clone()))
                .await;

            match result {
                Err(DownloadError::FileAlreadyExists(f)) if !overwrite => {
                    return Err(BootstrapDownloadError::Download(
                        DownloadError::FileAlreadyExists(f),
                    ));
                }
                Err(e) => return Err(BootstrapDownloadError::Download(e)),
                Ok(_) => {}
            }

            let size = fullpath
                .metadata()
                .map_err(|source| BootstrapDownloadError::ResourceIo {
                    url: res.url.to_owned(),
                    save_as: res.save_as.to_owned(),
                    action: "reading downloaded file metadata",
                    path: fullpath.clone(),
                    source,
                })?
                .len();
            let md5 = md5_file(&fullpath).map_err(|source| BootstrapDownloadError::ResourceIo {
                url: res.url.to_owned(),
                save_as: res.save_as.to_owned(),
                action: "hashing downloaded file",
                path: fullpath.clone(),
                source,
            })?;

            if let Some(ref pb) = aggregate_pb {
                pb.inc(1);
            }

            log_with_mp(&mp, || {
                info!(
                    "{} downloaded, size: {}, md5: {}, time: {:?}",
                    res.save_as,
                    size,
                    md5,
                    since.elapsed()
                );
            });

            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            if let Some(unzip_to) = res.unzip_to {
                let unzip_to_path = output_dir.join(unzip_to);
                if !unzip_to_path.exists() {
                    std::fs::create_dir_all(&unzip_to_path).map_err(|source| {
                        BootstrapDownloadError::ResourceIo {
                            url: res.url.to_owned(),
                            save_as: res.save_as.to_owned(),
                            action: "creating unzip target directory",
                            path: unzip_to_path.clone(),
                            source,
                        }
                    })?;
                }

                debug!("unzipping {} to {}", res.save_as, unzip_to);

                let file = std::fs::File::open(&fullpath).map_err(|source| {
                    BootstrapDownloadError::ResourceIo {
                        url: res.url.to_owned(),
                        save_as: res.save_as.to_owned(),
                        action: "opening downloaded zip file",
                        path: fullpath.clone(),
                        source,
                    }
                })?;
                let mut archive =
                    zip::ZipArchive::new(file).map_err(|source| BootstrapDownloadError::Unzip {
                        url: res.url.to_owned(),
                        save_as: res.save_as.to_owned(),
                        action: "reading zip archive",
                        source,
                    })?;
                archive
                    .extract_unwrapped_root_dir(unzip_to_path, zip::read::root_dir_common_filter)
                    .map_err(|source| BootstrapDownloadError::Unzip {
                        url: res.url.to_owned(),
                        save_as: res.save_as.to_owned(),
                        action: "extracting zip archive",
                        source,
                    })?;

                log_with_mp(&mp, || {
                    info!("{} unzipped to {}", res.save_as, unzip_to);
                });
            }

            Ok(())
        };

        tasks.push(task);

        // Limit the number of concurrent downloads
        if tasks.len() >= max_concurrent
            && let Some(result) = tasks.next().await
        {
            result?;
        }
    }

    // Process remaining tasks
    tasks.try_collect::<Vec<_>>().await?;

    if let Some(pb) = aggregate_pb {
        pb.finish_with_message("Downloading resources  done");
    }

    Ok(())
}

/// Web assets to download from CDN.
struct WebAsset {
    path: &'static str,
    cdn_kind: CdnKind,
}

#[derive(Clone, Copy)]
enum CdnKind {
    Gadgets,
    Game,
}

const WEB_ASSETS: &[WebAsset] = &[
    WebAsset {
        path: "gadget_html5/js/kcs_const.js",
        cdn_kind: CdnKind::Gadgets,
    },
    WebAsset {
        path: "kcs2/js/main.js",
        cdn_kind: CdnKind::Game,
    },
    WebAsset {
        path: "kcs2/version.json",
        cdn_kind: CdnKind::Game,
    },
];

/// Download key web assets (`kcs_const.js`, `main.js`, `version.json`) from CDN.
///
/// Each asset is fetched into a temporary file next to its destination and only
/// moved into place once the transfer succeeds, so a failed run never damages the
/// copy already on disk. Files that already exist are skipped unless `overwrite`
/// is true. If any asset cannot be fetched — because every CDN failed or because
/// none is configured — the whole call fails with
/// [`BootstrapDownloadError::WebAssetUnavailable`].
pub async fn download_web_assets(
    cache_root: &std::path::Path,
    gadgets_cdn: &[String],
    game_cdn: &[String],
    proxy: Option<&str>,
    overwrite: bool,
) -> Result<(), BootstrapDownloadError> {
    let client = Arc::new(new_reqwest_client(proxy, None).map_err(|source| {
        BootstrapDownloadError::ReqwestClient {
            proxy: proxy.map(ToOwned::to_owned),
            source,
        }
    })?);

    let mp = new_multi_progress();
    let mut failures: Vec<&'static str> = Vec::new();

    for asset in WEB_ASSETS {
        let (cdns, cdn_name) = match asset.cdn_kind {
            CdnKind::Gadgets => (gadgets_cdn, "gadgets_cdn"),
            CdnKind::Game => (game_cdn, "game_cdn"),
        };

        if cdns.is_empty() {
            log_with_mp(&mp, || {
                warn!(
                    "No {cdn_name} configured for {} — set {cdn_name} in emukc.config.toml, or pass --skip-web-assets.",
                    asset.path,
                );
            });
            failures.push(asset.path);
            continue;
        }

        let dest = cache_root.join(asset.path);
        if dest.exists() && !overwrite {
            debug!("Skipping {} — already exists", asset.path);
            continue;
        }

        // same directory as `dest`, so the rename below is atomic
        let tmp = dest.with_extension("part");
        let mut downloaded = false;

        for cdn in cdns {
            let cdn = cdn.trim_end_matches('/');
            let url = format!("http://{cdn}/{}", asset.path);

            let result = emukc_network::download::Request::builder()
                .url(&url)
                .save_as(&tmp)
                .overwrite(true)
                .skip_header_check(true)
                .build()?
                .execute(Some((*client).clone()))
                .await;

            match result {
                Ok(()) => {
                    std::fs::rename(&tmp, &dest)?;
                    log_with_mp(&mp, || {
                        info!("Downloaded {} from {}", asset.path, url);
                    });
                    downloaded = true;
                    break;
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&tmp);
                    log_with_mp(&mp, || {
                        warn!("Failed to download {} from {}: {e}", asset.path, url);
                    });
                }
            }
        }

        if !downloaded {
            log_with_mp(&mp, || {
                warn!("All CDN sources failed for {}", asset.path);
            });
            failures.push(asset.path);
        }
    }

    if let Some(first) = failures.first() {
        error!("Web assets could not be refreshed: {}", failures.join(", "));
        return Err(BootstrapDownloadError::WebAssetUnavailable {
            path: (*first).to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn web_assets_fail_when_no_cdn_is_configured() {
        let root = tempfile::tempdir().unwrap();

        let err = download_web_assets(root.path(), &[], &[], None, true).await.unwrap_err();

        assert!(
            matches!(err, BootstrapDownloadError::WebAssetUnavailable { .. }),
            "expected WebAssetUnavailable, got {err:?}"
        );
    }

    #[tokio::test]
    async fn failed_download_keeps_the_existing_file_and_leaves_no_temp() {
        let root = tempfile::tempdir().unwrap();
        let cdns = vec!["invalid.example.invalid".to_owned()];

        for asset in WEB_ASSETS {
            let dest = root.path().join(asset.path);
            std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
            std::fs::write(&dest, b"original").unwrap();
        }

        let err = download_web_assets(root.path(), &cdns, &cdns, None, true).await.unwrap_err();
        assert!(
            matches!(err, BootstrapDownloadError::WebAssetUnavailable { .. }),
            "expected WebAssetUnavailable, got {err:?}"
        );

        for asset in WEB_ASSETS {
            let dest = root.path().join(asset.path);
            assert_eq!(std::fs::read(&dest).unwrap(), b"original", "{} was modified", asset.path);
            assert!(
                !dest.with_extension("part").exists(),
                "temp file left behind for {}",
                asset.path
            );
        }
    }
}
