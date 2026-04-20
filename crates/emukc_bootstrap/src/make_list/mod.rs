use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use futures::{StreamExt, stream::FuturesUnordered};
use serde::{Deserialize, Serialize};

use emukc_cache::{IntoVersion, Kache, KacheError};
use emukc_model::codex::Codex;

use errors::CacheListMakingError;
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

pub mod config;
pub mod errors;
pub mod holes_report;
pub mod manifest;
pub mod progress;

mod source;

pub(crate) use source::kcs2::resources::slot::has_btxt_flat_coverage;

/// Strategy for making a cache list.
#[derive(Clone, Debug, PartialEq)]
pub enum CacheListMakeStrategy {
    /// Default strategy
    Default,
    /// Minimal strategy
    Minimal,
    /// Greedy strategy with configuration
    Greedy(config::GreedyConfig),
    /// Manifest strategy — uses resource_manifest.json
    Manifest,
}

/// A single cache list entry
#[derive(Debug, Serialize, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct CacheListItem {
    /// resource id
    #[serde(rename = "_id")]
    pub id: i64,

    /// resource path
    pub path: String,

    /// Resource version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug)]
pub(crate) struct CacheList {
    pub items: BTreeSet<CacheListItem>,

    next_id: i64,
}

impl CacheList {
    pub fn new() -> Self {
        Self {
            items: BTreeSet::new(),
            next_id: 0,
        }
    }

    pub fn add(&mut self, path: String, version: impl IntoVersion) -> &mut Self {
        let version = version.into_version();
        let item = CacheListItem {
            id: self.next_id,
            path,
            version,
        };
        self.items.insert(item);
        self.next_id += 1;

        self
    }

    pub fn add_unversioned(&mut self, path: String) -> &mut Self {
        let item = CacheListItem {
            id: self.next_id,
            path,
            version: None,
        };
        self.items.insert(item);
        self.next_id += 1;

        self
    }
}

/// Make a cache list.
///
/// # Arguments
///
/// * `mst` - The API manifest.
/// * `kache` - The kache instance.
/// * `outpath` - The output path.
/// * `overwrite` - Whether to overwrite the output file if it already exists.
///
/// # Returns
///
/// A `Result` containing either `Ok(())` if the cache list was successfully made, or an error if it failed.
pub async fn make(
    codex: &Codex,
    kache: &Kache,
    outpath: impl AsRef<std::path::Path>,
    strategy: CacheListMakeStrategy,
    overwrite: bool,
) -> Result<(), CacheListMakingError> {
    let out = outpath.as_ref().to_owned();
    if !overwrite && out.exists() {
        return Err(CacheListMakingError::FileExists(out));
    }

    info!("making cache list to {:?}", out);

    let mut list = CacheList::new();

    source::make(codex, kache, strategy.clone(), &mut list).await?;

    for item in list.items.iter() {
        let line = serde_json::to_string(item)?;
        debug!("{}", line);
    }

    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(&out).await?;
    for item in list.items.iter() {
        let line = serde_json::to_string(item)?;
        file.write_all(line.as_bytes()).await?;
        file.write_u8(b'\n').await?;
    }

    file.sync_all().await?;

    info!("cache list made to {:?}", out);

    // Generate holes report for greedy mode
    if matches!(strategy, CacheListMakeStrategy::Greedy(_)) {
        let holes_path = out.parent().unwrap_or(std::path::Path::new(".")).join("holes_report.txt");
        let holes = source::kcs2::resources::ship::get_holes_report();
        if !holes.is_empty() {
            let mut holes_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&holes_path)
                .await?;
            holes_file.write_all(b"// Generated holes report - copy to source files\n\n").await?;
            for hole in holes {
                holes_file.write_all(hole.as_bytes()).await?;
                holes_file.write_all(b"\n\n").await?;
            }
            holes_file.sync_all().await?;
            info!("holes report saved to {:?}", holes_path);
        }
        source::kcs2::resources::ship::clear_holes_report();
    }

    Ok(())
}

const MAX_CHECK_SIZE: usize = 32;

/// Helper to extract concurrent value from strategy
pub(crate) fn get_concurrent(strategy: &CacheListMakeStrategy) -> usize {
    match strategy {
        CacheListMakeStrategy::Greedy(config) => config.concurrent,
        _ => 16,
    }
}

/// Check if a list of URLs exist on the remote cache.
///
/// # Arguments
///
/// * `cache` - The remote cache to check against.
/// * `urls` - The list of URLs to check.
/// * `concurrent` - The maximum number of concurrent checks.
/// * `tracker` - Optional progress tracker.
///
/// # Returns
///
/// A `HashMap` mapping each URL to a boolean indicating whether it exists on the remote cache.
pub async fn batch_check_exists(
    cache: Arc<Kache>,
    mut urls: Vec<(String, String)>,
    concurrent: usize,
    tracker: Option<Arc<progress::ProgressTracker>>,
) -> Result<HashMap<(String, String), bool>, KacheError> {
    let q = concurrent.clamp(1, MAX_CHECK_SIZE);
    let mut result: HashMap<(String, String), bool> = HashMap::new();
    let mut tasks = FuturesUnordered::new();
    let mut check_count = 0;

    loop {
        while tasks.len() < q {
            match urls.pop() {
                Some((url, ver)) => {
                    let c = cache.clone();
                    let key = url.clone();
                    let t = tracker.clone();
                    tasks.push(async move {
                        let exists = c.exists_on_remote(&key, &ver).await?;
                        if let Some(tracker) = t {
                            tracker.increment_checked();
                            if exists {
                                tracker.increment_found();
                            }
                        }
                        Ok::<((String, String), bool), KacheError>(((key, ver), exists))
                    });
                }
                None => {
                    break;
                }
            }
        }

        if tasks.is_empty() {
            break;
        }

        match tasks.next().await {
            Some(Ok(((key, ver), exists))) => {
                result.insert((key, ver), exists);
                check_count += 1;
                if let Some(ref t) = tracker
                    && check_count % 100 == 0
                {
                    t.report();
                }
            }
            Some(Err(err)) => {
                return Err(err);
            }
            None => {
                break;
            }
        }
    }

    if let Some(ref t) = tracker {
        t.report();
    }

    Ok(result)
}
