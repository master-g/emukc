## Context

The cache populate pipeline lives at `crates/emukc_bootstrap/src/populate.rs`. It accepts a JSON-lines list of `(path, optional version)` entries, downloads each via `Kache`, retries failures in a second pass, and prints a summary. Recent commits added:

- `32aa4e1` — decoder-driven cache list IDs and filters.
- `e808a17` — dedup by `(path, version)` and fast-fail on 404.
- `d1335de` — indicatif progress bars for `populate`, `download`, `make-list`.
- `16c112f` — version-rollback detection: skip retrying entries whose new version differs from the disk copy by treating them as "already up to date" rather than as a download failure.

Commit `16c112f` added the rollback-skip logic, but did so by string-matching `f.error.contains("file version not matched")`. This is the dominant brittleness vector in the current implementation. At the same time, the older infrastructure (the `Mutex<Vec<FailedItem>>` failure aggregator, the `count_lines` pre-pass) was inherited from before the indicatif rework and was never reconsidered.

## Goals / Non-Goals

**Goals:**

- `populate.rs` SHALL classify pass-2 retry candidates via a typed match on `KacheError`, not by inspecting the error's `Display` output.
- `populate.rs` SHALL read `path_to_list` exactly once.
- The failure aggregation primitive SHALL be either `tokio::sync::Mutex` or a lock-free queue (`crossbeam_queue::SegQueue`); `std::sync::Mutex` SHALL NOT be held inside an async task body.
- Existing retry/skip behavior SHALL remain bit-identical from a user's perspective: version-rollback entries skip pass 2, all other failures retry.

**Non-Goals:**

- Restructuring the two-pass strategy.
- Adding new error variants to `KacheError`.
- Changing the JSONL list file format.
- Adjusting `MAX_CONCURRENT` or per-pass concurrency tuning.

## Decisions

### D1. `FailedItem.error: KacheError` (not `String`)

**Decision**: change `FailedItem` (in `crates/emukc_bootstrap/src/progress.rs`) to store `error: KacheError` directly. The print path uses `error.to_string()` at the boundary; the classification path matches on the variant.

**Alternative considered**: introduce a separate `FailureKind` enum (e.g., `Retry | SkipVersionRollback | Fatal`) and decide the kind at the call site immediately after the failed `opt.get(...)` future. Rejected — duplicates information already encoded in `KacheError`, and forces every new error variant to pick a kind in two places.

### D2. Drop `count_lines`; derive `total_files` from `all_items.len()`

**Decision**: remove the `count_lines` helper. Parse the list file into `Vec<(String, Option<String>)>` in one streaming pass; create the progress bar with `total_files = all_items.len() as u64` afterward. The progress bar appears slightly later (after parsing), but parsing is fast (sub-100ms for current list sizes).

**Alternative considered**: keep `count_lines` for early progress-bar instantiation. Rejected — the early bar shows zero progress until the parse pass completes anyway, so the UX gain is illusory.

### D3. Failure aggregation: `tokio::sync::Mutex<Vec<FailedItem>>`

**Decision**: switch the aggregator to `tokio::sync::Mutex<Vec<FailedItem>>`. Each task acquires the lock asynchronously to push a single failure. Contention is bounded by `MAX_CONCURRENT = 32` and the lock is held for ~ns scale.

**Alternative considered**:

- `crossbeam_queue::SegQueue<FailedItem>`: lock-free, MPMC. Slightly faster but `SegQueue::pop` returns one at a time, so the caller would loop to drain. Adds complexity for no measurable gain at 32-task concurrency.
- `parking_lot::Mutex`: blocking, would need `tokio::task::block_in_place` to be safe under multi-thread scheduler. Worse than `tokio::sync::Mutex`.

`tokio::sync::Mutex` is already a transitive dep via Tokio; no new crate required.

### D4. Classification match

**Decision**: replace `populate.rs:208` with:

```rust
let (skipped, retry_items): (Vec<_>, Vec<_>) =
    pass1_failures.into_iter().partition(|f| matches!(f.error, KacheError::InvalidFileVersion(_)));
```

This matches the structured variant directly. If `KacheError` ever grows new "not actually a download failure" variants, they are added here explicitly rather than inheriting via the `Display` string.

## Risks / Trade-offs

- [`FailedItem` field-type change ripples] → `print_populate_summary` and any test using `FailedItem` need updating. Mitigation: all consumers live in `emukc_bootstrap` itself; `cargo check -p emukc_bootstrap` finds them in one pass.
- [Error variant pattern is non-exhaustive] → if a future `KacheError` variant should *also* be skipped, we silently retry it. Mitigation: switch to `match` (not `matches!`) with all variants enumerated, so adding a new variant produces a compile error at this site.
- [Async mutex acquire on hot path] → at `MAX_CONCURRENT = 32` the lock is uncontended in practice; the overhead of `tokio::sync::Mutex::lock().await` is one yield point per failure (only failures, not successes). Acceptable.
- [Removing `count_lines` defers progress bar] → progress bar now shows after JSONL parse completes, ~tens of milliseconds later. Imperceptible in user testing.

## Migration Plan

1. Change `FailedItem.error` to `KacheError`. Update `failures.lock().unwrap().push(FailedItem { error: e, ... })` (no `.to_string()` call) and update `print_populate_summary` to call `failure.error.to_string()` at the print site.
2. Switch `Arc<std::sync::Mutex<Vec<FailedItem>>>` → `Arc<tokio::sync::Mutex<Vec<FailedItem>>>`. Make all push sites and the final drain `.lock().await`.
3. Replace the string-match partition with a `matches!(f.error, KacheError::InvalidFileVersion(_))` partition.
4. Delete `count_lines`. Move list parsing to before progress-bar construction. Use `all_items.len()` for `total_files`.
5. Run `cargo test -p emukc_bootstrap` and `cargo run -- bootstrap` against a small subset of resources to verify behavior.

Rollback: each step is its own commit; revert in reverse order if needed.
