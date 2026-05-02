## ADDED Requirements

### Requirement: Structured-error retry classification

The cache `populate` pass-2 retry classification SHALL determine retry vs. skip eligibility by matching on the structured `KacheError` variant carried in `FailedItem.error`, not by string-matching the `Display` representation.

#### Scenario: Version rollback skipped

- **WHEN** a pass-1 failure has `FailedItem.error == KacheError::InvalidFileVersion(_)`
- **THEN** the entry SHALL be added to the `skipped` partition
- **THEN** the entry SHALL NOT be retried in pass 2
- **THEN** a single `tracing::warn!` SHALL be emitted with the count of skipped items

#### Scenario: Other errors retried

- **WHEN** a pass-1 failure has any `FailedItem.error` other than `KacheError::InvalidFileVersion`
- **THEN** the entry SHALL be added to the `retry_items` partition
- **THEN** the entry SHALL be re-attempted exactly once in pass 2

#### Scenario: Display string change does not affect classification

- **WHEN** the `#[error]` Display string of `KacheError::InvalidFileVersion` is altered
- **THEN** the classification SHALL continue to skip rollback entries correctly
- **THEN** the populate test suite SHALL still pass without modification

### Requirement: FailedItem stores typed error

The `FailedItem` struct in `crates/emukc_bootstrap/src/progress.rs` SHALL carry the structured `KacheError` value (not a `String`). The `FailedItem` SHALL NOT lose error variant information when constructed.

#### Scenario: Error captured without Display flattening

- **WHEN** a download task in `run_pass` records a failure
- **THEN** the constructed `FailedItem` SHALL contain the original `KacheError` value, not `e.to_string()`
- **THEN** subsequent code SHALL be able to distinguish error variants via `match`

#### Scenario: Summary printer formats at boundary

- **WHEN** `print_populate_summary` formats a `FailedItem` for display
- **THEN** it SHALL call `failure.error.to_string()` at the print site (the only place `Display` is consumed)

### Requirement: Single-pass list file reading

The `populate` function in `crates/emukc_bootstrap/src/populate.rs` SHALL open the list file (`path_to_list`) exactly once. The total file count SHALL be derived from the parsed `all_items.len()`, not from a separate pre-pass.

#### Scenario: List file opened once

- **WHEN** `populate(kache, path_to_list, concurrent)` runs to completion
- **THEN** `tokio::fs::File::open(path_to_list)` SHALL be called at most once
- **THEN** the helper `count_lines` SHALL NOT exist

#### Scenario: total_files derived after parse

- **WHEN** the JSONL parse pass completes
- **THEN** `total_files = all_items.len()` SHALL be used to construct the progress bar

### Requirement: Async-aware failure aggregation

The failure aggregator inside `run_pass` SHALL NOT use `std::sync::Mutex` from inside async task bodies. The chosen primitive SHALL be either `tokio::sync::Mutex` or a lock-free MPMC queue (e.g., `crossbeam_queue::SegQueue`) so that lock acquisition cooperates with the Tokio scheduler.

#### Scenario: No std::sync::Mutex in async hot path

- **WHEN** `cargo clippy --workspace -- -W clippy::await_holding_lock` runs
- **THEN** no warning SHALL fire on the `populate.rs` failure aggregation code
- **THEN** `grep -n "std::sync::Mutex" crates/emukc_bootstrap/src/populate.rs` SHALL return zero matches

#### Scenario: Aggregation correct under concurrency

- **WHEN** 32 concurrent tasks each push a `FailedItem`
- **THEN** the aggregated vector SHALL contain exactly 32 items in the order pushed (or any deterministic order, depending on primitive)
- **THEN** no `FailedItem` SHALL be lost or duplicated
