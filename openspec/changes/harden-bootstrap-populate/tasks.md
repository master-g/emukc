## 1. FailedItem typed error

- [x] 1.1 Edit `crates/emukc_bootstrap/src/progress.rs`: change `FailedItem.error` from `String` to `KacheError`. Update derives if needed (`KacheError` is `Debug + Display`, may need wrapping if `Clone` not derived).
- [x] 1.2 If `KacheError` is not `Clone`, change `FailedItem` to wrap with `Arc<KacheError>` instead. Document the choice in the struct doc comment.
- [x] 1.3 Update every construction of `FailedItem` (currently in `populate.rs:90-94`) to pass the original error value, not `e.to_string()`.
- [x] 1.4 Update `print_populate_summary` (and any other consumer) to call `failure.error.to_string()` at the formatting boundary.
- [x] 1.5 Run `cargo check -p emukc_bootstrap` and resolve all sites where `f.error` was treated as `&str`.

## 2. Structured-error classification

- [x] 2.1 Edit `populate.rs:208`: replace `pass1_failures.into_iter().partition(|f| f.error.contains("file version not matched"))` with `pass1_failures.into_iter().partition(|f| matches!(f.error, KacheError::InvalidFileVersion(_)))`.
- [x] 2.2 Verify by writing a unit test in `crates/emukc_bootstrap/src/populate.rs` (or a new test module): construct a `FailedItem` with `KacheError::InvalidFileVersion(...)` and another with a non-version error, run the partition, assert the split is correct.
- [x] 2.3 Confirm by changing the `#[error("file version not matched: {0}")]` Display string to something else, running the test, and observing it still passes (then revert the experimental Display change).

## 3. Single-pass list file reading

- [x] 3.1 Delete the `count_lines` helper in `populate.rs:21-32`.
- [x] 3.2 Move the JSONL parse loop to run before progress-bar construction. Set `let total_files = all_items.len();`.
- [x] 3.3 Construct progress bars (`aggregate_pb`, `stats_pb`) using the post-parse `total_files`.
- [ ] 3.4 Run `cargo run -- cache populate` against a small list file and confirm the progress bar displays correctly with the expected total.

## 4. Async-aware failure aggregation

- [x] 4.1 Replace `Arc<std::sync::Mutex<Vec<FailedItem>>>` with `Arc<tokio::sync::Mutex<Vec<FailedItem>>>` (or `Arc<crossbeam_queue::SegQueue<FailedItem>>` if D3 alternative is chosen).
- [x] 4.2 Update push sites (`populate.rs:90`) to `failures.lock().await.push(...)`.
- [x] 4.3 Update drain at end of `run_pass` (`populate.rs:116`) to `failures.lock().await.clone()` (or `Arc::try_unwrap(...)` + `into_inner` for zero-clone drain).
- [x] 4.4 Run `cargo clippy --workspace -- -W clippy::await_holding_lock` and confirm clean.

## 5. BOOTSTRAP.md update

- [x] 5.1 Add a paragraph to BOOTSTRAP.md troubleshooting explaining the version-rollback skip behavior: when populate reports `skipping N items with version rollback`, this means the on-disk version is newer than the manifest version and the entry is correctly not retried.
- [x] 5.2 Note that retried failures (the `failed` count in the summary) are genuine download errors.

## 6. Verification

- [x] 6.1 Run `cargo test -p emukc_bootstrap` clean.
- [x] 6.2 Run `cargo build --release` clean.
- [ ] 6.3 Run `cargo run -- cache populate` end-to-end against a real list file. Confirm the summary classification (succeeded / retried / recovered / failed / skipped) matches expectations.
- [ ] 6.4 Run `openspec validate harden-bootstrap-populate --strict` clean.
