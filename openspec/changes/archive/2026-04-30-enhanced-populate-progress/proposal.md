## Why

Cache populate runs with stdout suppressed and a single `indicatif` progress bar showing only file count and ETA. Users have no visibility into concurrency utilization, which files are being downloaded, or whether errors are occurring. This makes diagnosing populate failures and tuning concurrency difficult.

## What Changes

- Add a stats line to the populate display showing active concurrent tasks and error count
- Add per-task spinners showing the resource path (`item.path`) for each in-flight download
- Track errors with an atomic counter; failed tasks display their error briefly in the spinner before clearing
- Use `MultiProgress` to compose the aggregate bar, stats line, and spinners

## Capabilities

### New Capabilities
- `populate-progress-display`: Enhanced progress display for cache populate showing concurrency, active downloads, and errors

### Modified Capabilities

## Impact

- `crates/emukc_bootstrap/src/progress.rs` — new stats bar style helpers
- `crates/emukc_bootstrap/src/populate.rs` — restructure to use `MultiProgress`, atomic counters, per-task spinners
- No API changes, no dependency additions
