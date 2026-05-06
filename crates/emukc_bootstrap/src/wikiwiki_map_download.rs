//! Manual wikiwiki.jp map download helpers for offline data generation workflows.

use std::{collections::BTreeSet, path::Path, str::FromStr, sync::Arc};

use emukc_model::kc2::start2::ApiManifest;
use emukc_network::{client::new_reqwest_client, download::Request};
use futures::{StreamExt, stream};

use crate::download::BootstrapDownloadError;

const WIKIWIKI_MAP_ROOT: &str = "wikiwiki_map";
const WIKIWIKI_PAGE_ROOT: &str = "https://wikiwiki.jp/kancolle";

#[derive(Debug, Default, Clone, Copy)]
/// Download counts collected during a manual wikiwiki sync.
pub struct WikiwikiMapDownloadStats {
    /// Number of HTML pages downloaded successfully.
    pub pages: usize,
    /// Number of map pages skipped or failed.
    pub failures: usize,
}

#[derive(Debug, Clone)]
/// Controls which wikiwiki map pages are fetched during a manual sync.
pub struct WikiwikiMapDownloadOptions {
    /// Maximum parallel request count used for page fetches.
    pub concurrent: Option<usize>,
    /// Optional map name allow-list, such as `1-1`.
    pub map_filter: Option<BTreeSet<String>>,
    /// Whether any failed page should abort the whole download.
    pub strict: bool,
}

impl Default for WikiwikiMapDownloadOptions {
    fn default() -> Self {
        Self {
            concurrent: Some(2),
            map_filter: None,
            strict: true,
        }
    }
}

/// Build the canonical wikiwiki page URL for a map such as `1-1`.
pub fn wikiwiki_map_page_url(map_name: &str) -> Option<String> {
    let (maparea_id, mapinfo_no) = parse_map_name(map_name)?;
    let area = match maparea_id {
        1 => "%E9%8E%AE%E5%AE%88%E5%BA%9C%E6%B5%B7%E5%9F%9F",
        2 => "%E5%8D%97%E8%A5%BF%E8%AB%B8%E5%B3%B6%E6%B5%B7%E5%9F%9F",
        3 => "%E5%8C%97%E6%96%B9%E6%B5%B7%E5%9F%9F",
        4 => "%E8%A5%BF%E6%96%B9%E6%B5%B7%E5%9F%9F",
        5 => "%E5%8D%97%E6%96%B9%E6%B5%B7%E5%9F%9F",
        6 => "%E4%B8%AD%E9%83%A8%E6%B5%B7%E5%9F%9F",
        7 => "%E5%8D%97%E8%A5%BF%E6%B5%B7%E5%9F%9F",
        _ => return None,
    };

    Some(format!("{WIKIWIKI_PAGE_ROOT}/{area}/{maparea_id}-{mapinfo_no}"))
}

/// Download wikiwiki map pages into `<dir>/wikiwiki_map/pages`.
pub async fn download_wikiwiki_map(
    dir: impl AsRef<Path>,
    overwrite: bool,
    proxy: Option<&str>,
    concurrent: Option<usize>,
) -> Result<WikiwikiMapDownloadStats, BootstrapDownloadError> {
    download_wikiwiki_map_with_options(
        dir,
        overwrite,
        proxy,
        WikiwikiMapDownloadOptions {
            concurrent,
            ..WikiwikiMapDownloadOptions::default()
        },
    )
    .await
}

/// Download wikiwiki map pages with explicit fetch options.
pub async fn download_wikiwiki_map_with_options(
    dir: impl AsRef<Path>,
    overwrite: bool,
    proxy: Option<&str>,
    options: WikiwikiMapDownloadOptions,
) -> Result<WikiwikiMapDownloadStats, BootstrapDownloadError> {
    let root = dir.as_ref().join(WIKIWIKI_MAP_ROOT);
    let pages_dir = root.join("pages");
    if !pages_dir.exists() {
        std::fs::create_dir_all(&pages_dir)?;
    }

    let manifest = read_manifest(dir.as_ref())?;
    let mut map_names = manifest
        .api_mst_mapinfo
        .iter()
        .filter(|map| (1..=7).contains(&map.api_maparea_id))
        .map(|map| format!("{}-{}", map.api_maparea_id, map.api_no))
        .collect::<Vec<_>>();
    map_names.sort();
    map_names.dedup();

    if let Some(map_filter) = &options.map_filter {
        let before = map_names.len();
        map_names.retain(|map_name| map_filter.contains(map_name));
        let excluded = before - map_names.len();
        if excluded > 0 {
            info!("map filter excluded {excluded} map(s), {before} -> {}", map_names.len());
        }
    }

    let client = Arc::new(new_reqwest_client(proxy, None).map_err(|source| {
        BootstrapDownloadError::ReqwestClient {
            proxy: proxy.map(ToOwned::to_owned),
            source,
        }
    })?);

    let max_concurrent = options.concurrent.unwrap_or(2).clamp(1, 8);
    let strict = options.strict;
    let mut stats = WikiwikiMapDownloadStats::default();
    let mut results = stream::iter(map_names.into_iter().map(|map_name| {
        let client = client.clone();
        let save_as = pages_dir.join(format!("{map_name}.html"));
        async move {
            let Some(url) = wikiwiki_map_page_url(&map_name) else {
                return if strict {
                    Err(BootstrapDownloadError::Generic(format!(
                        "wikiwiki map path is unsupported for {map_name}",
                    )))
                } else {
                    Ok::<_, BootstrapDownloadError>((map_name, false))
                };
            };

            match Request::builder()
                .url(url)
                .save_as(save_as)
                .overwrite(overwrite)
                .skip_header_check(true)
                .build()?
                .execute(Some((*client).clone()))
                .await
            {
                Ok(()) => Ok((map_name, true)),
                Err(error) => {
                    if strict {
                        return Err(BootstrapDownloadError::Generic(format!(
                            "failed to download wikiwiki map {map_name}: {error}",
                        )));
                    }
                    warn!("skipping wikiwiki map {}: {}", map_name, error);
                    Ok((map_name, false))
                }
            }
        }
    }))
    .buffer_unordered(max_concurrent);

    while let Some(result) = results.next().await {
        let (_, downloaded) = result?;
        if downloaded {
            stats.pages += 1;
        } else {
            stats.failures += 1;
        }
    }

    Ok(stats)
}

fn parse_map_name(map_name: &str) -> Option<(i64, i64)> {
    let (maparea_id, mapinfo_no) = map_name.split_once('-')?;
    Some((maparea_id.parse().ok()?, mapinfo_no.parse().ok()?))
}

#[allow(clippy::result_large_err)]
fn read_manifest(root: &Path) -> Result<ApiManifest, BootstrapDownloadError> {
    let manifest_path = root.join("start2.json");
    let raw = std::fs::read_to_string(&manifest_path)?;
    ApiManifest::from_str(&raw).map_err(|source| BootstrapDownloadError::Json {
        path: manifest_path,
        source,
    })
}
