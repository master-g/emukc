use std::{
    path::{Path, PathBuf},
    sync::Arc,
    sync::atomic::{AtomicUsize, Ordering},
    time::Instant,
};

use futures::{StreamExt, stream::FuturesUnordered};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::make_list::CacheListItem;
use crate::progress::{
    FailedItem, PopulateStats, log_with_mp, new_multi_progress, new_progress_bar,
    new_progress_bar_on_mp, new_stats_bar, new_stats_bar_on_mp, populate_style,
    print_populate_summary, update_stats_message,
};
use emukc_cache::{GetOption, Kache, KacheError};

const MAX_CONCURRENT: usize = 32;

/// How a failed item should be treated after a pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureKind {
    /// The local copy is newer than the list asks for. Not a download failure;
    /// `get` already served the newer file, so there is nothing to retry.
    Rollback,
    /// A CDN answered 404. Retrying asks the same question again, so these are
    /// kept out of the retry queue and reported separately.
    Missing,
    /// Anything else: no CDN gave an answer, so the question is still open.
    Retryable,
}

fn classify_failure(error: &KacheError) -> FailureKind {
    match error {
        KacheError::InvalidFileVersion(_) => FailureKind::Rollback,
        KacheError::FileNotFound(_) => FailureKind::Missing,
        _ => FailureKind::Retryable,
    }
}

/// Sibling list path for one failure class, e.g. `cache_resources.failed.nedb`.
///
/// A class marker already on the input is replaced rather than appended, so
/// re-running against `cache_resources.failed.nedb` overwrites that file
/// instead of growing `cache_resources.failed.failed.nedb` — which nothing
/// would ever clean up, because cleanup derives its target from the input name.
fn failure_list_path(src: &Path, kind: &str) -> PathBuf {
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("nedb");
    let stem = src.file_stem().and_then(|e| e.to_str()).unwrap_or("cache_resources");
    let base =
        stem.strip_suffix(".failed").or_else(|| stem.strip_suffix(".missing")).unwrap_or(stem);
    src.with_file_name(format!("{base}.{kind}.{ext}"))
}

/// True when this path is itself one of the class lists we write.
fn carries_class_marker(path: &Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|stem| stem.ends_with(".failed") || stem.ends_with(".missing"))
}

/// Serialize failures in the input list's own JSONL format, so the result can
/// be fed straight back via `--src`.
fn failures_to_jsonl(failures: &[FailedItem]) -> Result<String, serde_json::Error> {
    let mut out = String::new();
    for f in failures {
        let item = CacheListItem {
            path: f.path.clone(),
            version: f.version.clone(),
        };
        out.push_str(&serde_json::to_string(&item)?);
        out.push('\n');
    }
    Ok(out)
}

/// Write `failures` to the class list beside `src`, or remove a stale one when
/// this run produced none. Neither failure here masks the run's own outcome.
async fn persist_failure_list(src: &Path, kind: &str, failures: &[FailedItem]) -> Option<PathBuf> {
    let path = failure_list_path(src, kind);
    if failures.is_empty() {
        // A run against one class's list owns only that class. The first thing
        // we print after a failure is `--src <failed list>`, and following it
        // must not delete the missing list from the full run — that file is the
        // only readable record of what is absent upstream, and rebuilding it
        // costs another full pass.
        if path != src && carries_class_marker(src) {
            return None;
        }
        if tokio::fs::try_exists(&path).await.unwrap_or(false)
            && let Err(e) = tokio::fs::remove_file(&path).await
        {
            warn!("could not remove stale {kind} list {}: {e}", path.display());
        }
        return None;
    }
    match failures_to_jsonl(failures) {
        Ok(body) => match tokio::fs::write(&path, body).await {
            Ok(()) => Some(path),
            Err(e) => {
                error!("could not write {kind} list {}: {e}", path.display());
                None
            }
        },
        Err(e) => {
            error!("could not serialize {kind} list: {e}");
            None
        }
    }
}

async fn run_pass(
    kache: &Arc<Kache>,
    items: Vec<(String, Option<String>)>,
    concurrent: usize,
    aggregate_pb: &Option<Arc<indicatif::ProgressBar>>,
    stats_pb: &Option<Arc<indicatif::ProgressBar>>,
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

            active_count.fetch_add(1, Ordering::Relaxed);

            let task = async move {
                let result = opt.get(&kache_clone, &item_path, version.clone()).await;

                if let Some(ref pb) = aggregate_pb {
                    pb.inc(1);
                }

                let active = active_count.fetch_sub(1, Ordering::Relaxed) - 1;

                if let Err(e) = result {
                    error_count.fetch_add(1, Ordering::Relaxed);
                    failures.lock().await.push(FailedItem {
                        path: item_path,
                        version,
                        error: Arc::new(e),
                    });
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
    let src = path_to_list.as_ref();

    let file = tokio::fs::File::open(src).await?;
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
        run_pass(&kache, all_items, q, &aggregate_pb, &stats_pb, &active_count).await;
    debug_assert_eq!(
        active_count.load(Ordering::Relaxed),
        0,
        "active_count must be 0 after run_pass"
    );

    let succeeded = total_files - pass1_failures.len();

    if pass1_failures.is_empty() {
        // Nothing failed, so any list left by an earlier run is stale.
        persist_failure_list(src, "failed", &[]).await;
        persist_failure_list(src, "missing", &[]).await;

        let stats = PopulateStats {
            total: total_files,
            succeeded,
            retried: 0,
            recovered: 0,
            failed: 0,
            missing: 0,
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

    // Pass 2 retries only what is still an open question. A rollback was already
    // served from the newer local file, and a 404 has been answered — asking the
    // same CDN again just spends a request to get the same 404.
    let mut rollback_count = 0usize;
    let mut missing: Vec<FailedItem> = Vec::new();
    let mut retryable: Vec<FailedItem> = Vec::new();
    for f in pass1_failures {
        match classify_failure(f.error.as_ref()) {
            FailureKind::Rollback => rollback_count += 1,
            FailureKind::Missing => missing.push(f),
            FailureKind::Retryable => retryable.push(f),
        }
    }
    if rollback_count > 0 {
        warn!("skipping {rollback_count} items with version rollback");
    }
    let retry_items: Vec<(String, Option<String>)> =
        retryable.into_iter().map(|f| (f.path, f.version)).collect();
    let retry_count = retry_items.len();

    if let Some(ref pb) = aggregate_pb {
        pb.set_length(total_files as u64 + retry_count as u64);
        pb.set_message(format!("Populating cache (retry {retry_count} items)"));
    }
    if let Some(ref pb) = stats_pb {
        pb.reset_elapsed();
    }

    let pass2_failures =
        run_pass(&kache, retry_items, q, &aggregate_pb, &stats_pb, &active_count).await;
    debug_assert_eq!(
        active_count.load(Ordering::Relaxed),
        0,
        "active_count must be 0 after run_pass"
    );

    let recovered = retry_count - pass2_failures.len();

    // An item can be answered 404 on the retry after a first-pass timeout, so
    // split pass 2's failures the same way rather than assuming they are all
    // retryable. Rollbacks cannot appear here (a rollback is served, not failed).
    let mut failed: Vec<FailedItem> = Vec::new();
    for f in pass2_failures {
        match classify_failure(f.error.as_ref()) {
            FailureKind::Missing => missing.push(f),
            _ => failed.push(f),
        }
    }
    let failed_count = failed.len();
    let missing_count = missing.len();

    let failed_list = persist_failure_list(src, "failed", &failed).await;
    let missing_list = persist_failure_list(src, "missing", &missing).await;

    let stats = PopulateStats {
        total: total_files,
        succeeded,
        retried: retry_count,
        recovered,
        failed: failed_count,
        missing: missing_count,
        elapsed: start.elapsed(),
    };

    print_populate_summary(&mp, &stats, &failed);

    if failed_count + missing_count > 0 {
        log_with_mp(&mp, || {
            // A list that could not be written leaves the summary's count with
            // nothing behind it, so fall back to printing the paths themselves
            // rather than sending the user back to the log file.
            if failed_list.is_none() && !failed.is_empty() {
                eprintln!("could not write the retry list; failed paths:");
                for f in &failed {
                    eprintln!("  ✗ {}", f.path);
                }
            }
            if missing_list.is_none() && !missing.is_empty() {
                eprintln!("could not write the missing list; 404 paths:");
                for f in &missing {
                    eprintln!("  ✗ {}", f.path);
                }
            }
            if let Some(path) = &failed_list {
                eprintln!("retry with: cache populate --src {}", path.display());
            }
            if let Some(path) = &missing_list {
                eprintln!("missing upstream (404), listed in: {}", path.display());
                eprintln!(
                    "  confirm across mirrors before using it to seed a hole table \
                     — a 404 here is one mirror's answer, not every mirror's"
                );
            }
        });

        if let Some(pb) = aggregate_pb {
            pb.finish_with_message(format!(
                "Populating cache  done ({total_files} files, {failed_count} failures, \
                 {missing_count} missing)"
            ));
        }
        if let Some(pb) = stats_pb {
            pb.finish_and_clear();
        }
        return Err(KacheError::InvalidFile(format!(
            "{failed_count} items failed after retry, {missing_count} missing (404)"
        )));
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
    use std::path::Path;
    use std::sync::Arc;

    use super::{
        FailureKind, classify_failure, failure_list_path, failures_to_jsonl, persist_failure_list,
    };
    use crate::make_list::CacheListItem;
    use crate::progress::FailedItem;
    use emukc_cache::KacheError;

    fn failed(path: &str, version: Option<&str>, error: KacheError) -> FailedItem {
        FailedItem {
            path: path.into(),
            version: version.map(Into::into),
            error: Arc::new(error),
        }
    }

    /// The retry queue is built from this, so a 404 landing in `Retryable` would
    /// spend a second request to be told the same thing, and a transient failure
    /// landing in `Missing` would never be retried at all.

    #[test]
    fn classify_failure_keeps_the_three_outcomes_apart() {
        assert_eq!(
            classify_failure(&KacheError::InvalidFileVersion("v1".into())),
            FailureKind::Rollback
        );
        assert_eq!(
            classify_failure(&KacheError::FileNotFound("kcs2/x.png".into())),
            FailureKind::Missing
        );
        assert_eq!(classify_failure(&KacheError::FailedOnAllCdn), FailureKind::Retryable);
        assert_eq!(
            classify_failure(&KacheError::InvalidFile("corrupt".into())),
            FailureKind::Retryable
        );
    }

    /// Feeding a failure list back through `--src` must not grow a new suffix
    /// each round: cleanup derives its target from the input name, so a stacked
    /// `*.failed.failed.nedb` would never be removed.
    #[test]
    fn failure_list_suffix_does_not_stack() {
        let src = Path::new("/z/cache/cache_resources.nedb");
        assert_eq!(
            failure_list_path(src, "failed"),
            Path::new("/z/cache/cache_resources.failed.nedb")
        );
        assert_eq!(
            failure_list_path(src, "missing"),
            Path::new("/z/cache/cache_resources.missing.nedb")
        );

        let retry = Path::new("/z/cache/cache_resources.failed.nedb");
        assert_eq!(failure_list_path(retry, "failed"), retry, "overwrites in place");
        assert_eq!(
            failure_list_path(retry, "missing"),
            Path::new("/z/cache/cache_resources.missing.nedb"),
            "the other class is derived from the base name, not appended"
        );
    }

    /// The written list is only useful if `--src` can read it back, which means
    /// it has to be the same JSONL shape as the input list.
    #[test]
    fn failures_serialize_into_readable_list_items() {
        let items = vec![
            failed("kcs2/a.png", Some("6.3.5.0"), KacheError::FailedOnAllCdn),
            failed("kcs2/b.png", None, KacheError::FailedOnAllCdn),
        ];

        let jsonl = failures_to_jsonl(&items).unwrap();
        let parsed: Vec<CacheListItem> =
            jsonl.lines().map(|l| serde_json::from_str(l).unwrap()).collect();

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].path, "kcs2/a.png");
        assert_eq!(parsed[0].version.as_deref(), Some("6.3.5.0"));
        assert_eq!(parsed[1].path, "kcs2/b.png");
        assert_eq!(parsed[1].version, None);

        // `skip_serializing_if` must hold, or the unversioned entry would come
        // back as `"version": null` and no longer match the input list's shape.
        assert!(!jsonl.lines().nth(1).unwrap().contains("version"));
    }

    #[test]
    fn empty_failure_set_serializes_to_nothing() {
        assert_eq!(failures_to_jsonl(&[]).unwrap(), "");
    }

    /// Retrying the failed list must not take the missing list with it. The
    /// command we print on failure is exactly `--src <failed list>`, and the
    /// missing list is the only readable record of what is absent upstream.
    #[tokio::test]
    async fn retrying_one_class_list_leaves_the_other_alone() {
        let dir = tempfile::TempDir::new().unwrap();
        let failed_list = dir.path().join("cache_resources.failed.nedb");
        let missing_list = dir.path().join("cache_resources.missing.nedb");
        tokio::fs::write(&failed_list, "{\"path\":\"kcs2/a.png\"}\n").await.unwrap();
        tokio::fs::write(&missing_list, "{\"path\":\"kcs2/b.png\"}\n").await.unwrap();

        // Re-running the failed list, and this time everything succeeded.
        persist_failure_list(&failed_list, "failed", &[]).await;
        persist_failure_list(&failed_list, "missing", &[]).await;

        assert!(!failed_list.exists(), "its own class list is cleared");
        assert!(missing_list.exists(), "the other class list is untouched");
    }

    /// A run against the real list does own both classes, so a clean run there
    /// still clears whatever an earlier run left behind.
    #[tokio::test]
    async fn a_clean_full_run_clears_both_class_lists() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("cache_resources.nedb");
        let failed_list = dir.path().join("cache_resources.failed.nedb");
        let missing_list = dir.path().join("cache_resources.missing.nedb");
        tokio::fs::write(&src, "{\"path\":\"kcs2/a.png\"}\n").await.unwrap();
        tokio::fs::write(&failed_list, "stale").await.unwrap();
        tokio::fs::write(&missing_list, "stale").await.unwrap();

        persist_failure_list(&src, "failed", &[]).await;
        persist_failure_list(&src, "missing", &[]).await;

        assert!(!failed_list.exists());
        assert!(!missing_list.exists());
    }

    /// What gets written has to survive the round trip, since the whole point
    /// is feeding it back through `--src`.
    #[tokio::test]
    async fn a_written_list_reads_back_as_list_items() {
        let dir = tempfile::TempDir::new().unwrap();
        let src = dir.path().join("cache_resources.nedb");
        let items = vec![failed("kcs2/a.png", Some("6.3.5.0"), KacheError::FailedOnAllCdn)];

        let written = persist_failure_list(&src, "failed", &items).await.unwrap();
        let body = tokio::fs::read_to_string(&written).await.unwrap();
        let parsed: Vec<CacheListItem> =
            body.lines().map(|l| serde_json::from_str(l).unwrap()).collect();

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path, "kcs2/a.png");
        assert_eq!(parsed[0].version.as_deref(), Some("6.3.5.0"));
    }
}
