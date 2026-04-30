## Why

CLI operations like `cache populate`, `bootstrap`, and `cache make-list` produce unstructured output: raw `tracing` log lines mixed with hand-rolled `\r` progress counters. This makes it hard to gauge progress, speed, or ETA at a glance — the experience feels primitive compared to modern CLI tools like brew, bun, or pnpm.

## What Changes

- Add `indicatif` crate for structured terminal progress bars with ETA, speed, and percentage
- Replace `print_progress()` in `populate.rs` with `indicatif::ProgressBar`
- Replace `ProgressTracker` in `make_list/progress.rs` with `indicatif::ProgressBar`
- Add `indicatif::MultiProgress` to `download.rs` for concurrent download visibility
- Add phase-labeled progress output to `bootstrap.rs`
- Use `MultiProgress::suspend()` to prevent log lines from clobbering progress bars

## Capabilities

### New Capabilities
- `cli-progress`: Structured progress reporting for long-running CLI operations (populate, bootstrap, make-list)

### Modified Capabilities

## Non-goals

- No full TUI framework (ratatui/crossterm) — indicatif's inline progress bars are sufficient
- No `tracing-indicatif` layer integration — the concurrent task model (FuturesUnordered + AtomicUsize) doesn't map cleanly to span-per-progress-bar
- No changes to server-mode output (only affects CLI commands)
- No interactive controls (pause/resume/cancel via keyboard)
- No changes to the `emukc_log` tracing subscriber pipeline

## Impact

- **Dependencies**: Add `indicatif` to workspace `Cargo.toml`, imported by `emukc_bootstrap`
- **Code**: `populate.rs`, `download.rs`, `make_list/progress.rs`, `bootstrap.rs` in `emukc_bootstrap`
- **No API changes**: This is purely a CLI presentation improvement
- **No breaking changes**: Command flags and behavior remain identical
