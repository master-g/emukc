use std::{
    sync::Arc,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use futures::{StreamExt, stream::FuturesUnordered};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::make_list::CacheListItem;
use crate::progress::{
    FailedItem, PopulateStats, new_multi_progress, new_progress_bar, new_progress_bar_on_mp,
    new_spinner_on_mp, new_stats_bar, new_stats_bar_on_mp, populate_style, print_populate_summary,
    update_stats_message,
};
use emukc_cache::{GetOption, Kache, KacheError};

const MAX_CONCURRENT: usize = 32;

async fn run_pass(
    kache: &Arc<Kache>,
    items: Vec<(String, Option<String>)>,
    concurrent: usize,
    aggregate_pb: &Option<Arc<indicatif::ProgressBar>>,
    stats_pb: &Option<Arc<indicatif::ProgressBar>>,
    mp: &Arc<Option<indicatif::MultiProgress>>,
    active_count: &Arc<AtomicUsize>,
) -> Vec<FailedItem> {
    let q = concurrent.clamp(1, MAX_CONCURRENT);
    let error_count = Arc::new(AtomicUsize::new(0));
    let failures: Arc<tokio::sync::Mutex<Vec<FailedItem>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let mut tasks = FuturesUnordered::new();

    let mut item_iter = items.into_iter();

    loop {
        while tasks.len() < q {
            let Some((item_path, version)) = item_iter.next() else {
                break;
            };

            let opt = GetOption::default().disable_mod();
            let kache_clone = kache.clone();
            let aggregate_pb = aggregate_pb.clone();
            let active_count = active_count.clone();
            let error_count = error_count.clone();
            let stats_pb = stats_pb.clone();
            let failures = failures.clone();

            let spinner = mp.as_ref().as_ref().map(|mp| new_spinner_on_mp(&item_path, mp));

            active_count.fetch_add(1, Ordering::Relaxed);

            let task = async move {
                let result = opt.get(&kache_clone, &item_path, version.clone()).await;

                if let Some(ref pb) = aggregate_pb {
                    pb.inc(1);
                }

                let active = active_count.fetch_sub(1, Ordering::Relaxed) - 1;

                match result {
                    Ok(_) => {
                        if let Some(sp) = spinner {
                            sp.finish_and_clear();
                        }
                    }
                    Err(e) => {
                        error_count.fetch_add(1, Ordering::Relaxed);
                        if let Some(sp) = spinner {
                            sp.finish_and_clear();
                        }
                        failures.lock().await.push(FailedItem {
                            path: item_path,
                            version,
                            error: Arc::new(e),
                        });
                    }
                }

                if let Some(ref pb) = stats_pb {
                    let errors = error_count.load(Ordering::Relaxed);
                    update_stats_message(pb, active, q, errors);
                }
            };

            tasks.push(task);
        }

        if tasks.is_empty() {
            break;
        }

        if tasks.next().await.is_none() {
            break;
        }
    }

    failures.lock().await.clone()
}

/// Populate the cache with the list file.
///
/// # Arguments
///
/// * `kache` - The kache instance.
/// * `path_to_list` - The path to the list file.
/// * `concurrent` - The number of concurrent downloads.
pub async fn populate(
    kache: Arc<Kache>,
    path_to_list: impl AsRef<std::path::Path>,
    concurrent: usize,
) -> Result<(), KacheError> {
    let start = Instant::now();

    let file = tokio::fs::File::open(&path_to_list).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut all_items: Vec<(String, Option<String>)> = Vec::new();

    loop {
        let Some(line) = lines.next_line().await? else {
            break;
        };
        let item: CacheListItem =
            serde_json::from_str(&line).map_err(|e| KacheError::InvalidFile(e.to_string()))?;
        all_items.push((item.path, item.version));
    }

    let total_files = all_items.len();

    let q = concurrent.clamp(1, MAX_CONCURRENT);
    let active_count = Arc::new(AtomicUsize::new(0));

    let mp = Arc::new(new_multi_progress());
    let aggregate_pb = match mp.as_ref() {
        Some(mp_ref) => {
            let pb = new_progress_bar_on_mp(
                total_files as u64,
                "Populating cache",
                populate_style(),
                mp_ref,
            );
            Some(Arc::new(pb))
        }
        None => {
            new_progress_bar(total_files as u64, "Populating cache", populate_style()).map(Arc::new)
        }
    };
    let stats_pb = match mp.as_ref() {
        Some(mp_ref) => {
            let pb = new_stats_bar_on_mp(q, mp_ref);
            Some(Arc::new(pb))
        }
        None => new_stats_bar(q).map(Arc::new),
    };

    // Pass 1
    let pass1_failures =
        run_pass(&kache, all_items, q, &aggregate_pb, &stats_pb, &mp, &active_count).await;
    debug_assert_eq!(
        active_count.load(Ordering::Relaxed),
        0,
        "active_count must be 0 after run_pass"
    );

    let succeeded = total_files - pass1_failures.len();

    if pass1_failures.is_empty() {
        let stats = PopulateStats {
            total: total_files,
            succeeded,
            retried: 0,
            recovered: 0,
            failed: 0,
            elapsed: start.elapsed(),
        };
        let final_failures: Vec<FailedItem> = Vec::new();
        print_populate_summary(&mp, &stats, &final_failures);

        if let Some(pb) = aggregate_pb {
            pb.finish_with_message(format!("Populating cache  done ({total_files} files)"));
        }
        if let Some(pb) = stats_pb {
            pb.finish_and_clear();
        }
        return Ok(());
    }

    // Pass 2: retry failed items (excluding InvalidFileVersion — version rollback is not a download failure)
    let (skipped, retry_items): (Vec<_>, Vec<_>) = pass1_failures
        .into_iter()
        .partition(|f| matches!(f.error.as_ref(), KacheError::InvalidFileVersion(_)));
    if !skipped.is_empty() {
        warn!("skipping {} items with version rollback", skipped.len());
    }
    let retry_items: Vec<(String, Option<String>)> =
        retry_items.into_iter().map(|f| (f.path, f.version)).collect();
    let retry_count = retry_items.len();

    if let Some(ref pb) = aggregate_pb {
        pb.set_length(total_files as u64 + retry_count as u64);
        pb.set_message(format!("Populating cache (retry {retry_count} items)"));
    }
    if let Some(ref pb) = stats_pb {
        pb.reset_elapsed();
    }

    let pass2_failures =
        run_pass(&kache, retry_items, q, &aggregate_pb, &stats_pb, &mp, &active_count).await;
    debug_assert_eq!(
        active_count.load(Ordering::Relaxed),
        0,
        "active_count must be 0 after run_pass"
    );

    let recovered = retry_count - pass2_failures.len();
    let failed_count = pass2_failures.len();

    let stats = PopulateStats {
        total: total_files,
        succeeded,
        retried: retry_count,
        recovered,
        failed: failed_count,
        elapsed: start.elapsed(),
    };

    print_populate_summary(&mp, &stats, &pass2_failures);

    if failed_count > 0 {
        if let Some(pb) = aggregate_pb {
            pb.finish_with_message(format!(
                "Populating cache  done ({total_files} files, {failed_count} failures)"
            ));
        }
        if let Some(pb) = stats_pb {
            pb.finish_and_clear();
        }
        return Err(KacheError::InvalidFile(format!("{failed_count} items failed after retry")));
    }

    if let Some(pb) = aggregate_pb {
        pb.finish_with_message(format!(
            "Populating cache  done ({total_files} files, {retry_count} retried, all recovered)"
        ));
    }
    if let Some(pb) = stats_pb {
        pb.finish_and_clear();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::progress::FailedItem;
    use emukc_cache::KacheError;

    #[test]
    fn partition_by_error_variant() {
        let items = vec![
            FailedItem {
                path: "a".into(),
                version: None,
                error: Arc::new(KacheError::InvalidFileVersion("v1".into())),
            },
            FailedItem {
                path: "b".into(),
                version: None,
                error: Arc::new(KacheError::FileNotFound("missing".into())),
            },
            FailedItem {
                path: "c".into(),
                version: Some("v2".into()),
                error: Arc::new(KacheError::InvalidFileVersion("v2".into())),
            },
            FailedItem {
                path: "d".into(),
                version: None,
                error: Arc::new(KacheError::FailedOnAllCdn),
            },
        ];

        let (skipped, retry): (Vec<_>, Vec<_>) = items
            .into_iter()
            .partition(|f| matches!(f.error.as_ref(), KacheError::InvalidFileVersion(_)));

        assert_eq!(skipped.len(), 2);
        assert_eq!(retry.len(), 2);
        assert!(
            skipped.iter().all(|f| matches!(f.error.as_ref(), KacheError::InvalidFileVersion(_)))
        );
        assert!(
            retry.iter().all(|f| !matches!(f.error.as_ref(), KacheError::InvalidFileVersion(_)))
        );
    }
}
