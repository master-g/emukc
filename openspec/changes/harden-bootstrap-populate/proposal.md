## Why

Recent bootstrap commits (32aa4e1, e808a17, 16c112f, d1335de) progressively hardened the cache pipeline, but a follow-up audit found three brittleness issues in `crates/emukc_bootstrap/src/populate.rs` that defeat parts of the hardening work:

1. **String-matched error classification.** `populate.rs:208` decides whether to retry vs. skip a failed item by `f.error.contains("file version not matched")`. The structured `KacheError::InvalidFileVersion(String)` variant is lost at `populate.rs:93` because the error is collapsed to `e.to_string()` before being stored in `FailedItem`. Any future change to the `#[error]` Display string silently flips every rollback into a retried download.
2. **Blocking `std::sync::Mutex` inside async tasks.** `populate.rs:45-46, 90, 116` use a `std::sync::Mutex<Vec<FailedItem>>` from inside futures driven by Tokio's multi-threaded scheduler. With `MAX_CONCURRENT = 32` this rarely contends, but the pattern is wrong — `parking_lot::Mutex` or `tokio::sync::Mutex` would be correct, and a lock-free `crossbeam_queue::SegQueue` would be best.
3. **Double-read of `path_to_list`.** `populate(...)` opens the list file once via `count_lines` to count lines (`populate.rs:132`), then re-opens it (`populate.rs:134`) to parse JSON entries. The first read is purely for `total_files`, which can be derived from `all_items.len()` after the parse pass. Half the file I/O is wasted, and on slow disks the two reads can disagree if the file is touched mid-bootstrap.

A fourth, smaller concern: `failures.lock().unwrap().clone()` at `populate.rs:116` clones the entire failure vector at the end of each pass; replacing with `Arc::try_unwrap` + `into_inner` (or moving to `crossbeam_queue::SegQueue::pop` draining) would avoid the clone.

## What Changes

- **Preserve structured error variants**: change `FailedItem.error` from `String` to `KacheError` (or `Arc<KacheError>` if cloning is needed). Update consumers (`print_populate_summary`, retry classifier).
- **Replace string-match classification**: switch `populate.rs:208` to a typed match on `KacheError::InvalidFileVersion(_) => skipped, _ => retry`.
- **Single-pass file read**: drop `count_lines`. Parse the list file once, count `all_items.len()` for `total_files`, and create the progress bar after parsing.
- **Lock-free or async-aware failure aggregation**: replace `Arc<std::sync::Mutex<Vec<FailedItem>>>` with `crossbeam_queue::SegQueue<FailedItem>` (already a dependency in the workspace) or `tokio::sync::Mutex` if collection ordering matters.
- **Avoid Vec clone at pass boundary**: drain the queue/lock once into an owned `Vec<FailedItem>` for return.

## Capabilities

### Modified Capabilities

- `bootstrap-guide`: 添加文档说明 populate 命令在结构化错误分类下的重试/跳过判定（version rollback 跳过，其它错误重试）。

### New Capabilities

- `populate-error-classification`: defines the structured-error contract that `populate.rs` SHALL honor when deciding pass-2 retry vs. skip.

## Non-goals

- Restructuring the two-pass retry strategy itself. The pass-1 / pass-2 split is fine; only the classification mechanism changes.
- Replacing `tokio::fs::File` with `std::fs::File` blocking I/O.
- Adding new download backends to `Kache`.
- Adding a third pass.

## Impact

- **Affected crate**: `emukc_bootstrap` only. `emukc_cache::KacheError` already exposes the variants needed; no change required there.
- **Public API**: `FailedItem` is re-exported from `crates/emukc_bootstrap/src/progress.rs` — consumers in the binary are limited to `print_populate_summary`. Field-type change is a breaking change but the symbol is internal.
- **Performance**: dropping `count_lines` saves one full file scan (~600 KiB JSONL today, growing). Lock-free queue eliminates contention on aggregate throughput.
- **No DB schema changes, no Codex changes, no KCSAPI changes.**
