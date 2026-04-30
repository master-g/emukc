## Why

Code review of the indicatif-progress-bars change revealed two bugs and several maintenance risks: the aggregate progress bar overflows past 100% during pass 2 retries, `Arc::try_unwrap` can panic on task failure, and the `mp.suspend` log pattern is duplicated ~8 times across download.rs. These need fixing before merge.

## What Changes

- Fix aggregate progress bar length tracking: reset or extend the bar before pass 2 so it never shows >100%
- Harden `run_pass` failure collection: replace `Arc::try_unwrap().unwrap()` with a safe fallback
- Extract a `log_with_mp` helper in `progress.rs` to eliminate the repeated `if let Some(mp) { mp.suspend(|| { log!() }) } else { log!() }` pattern
- Fix mixed tab/space indentation in download.rs suspend blocks
- Add active_count invariant assertion between passes in populate.rs

## Capabilities

### New Capabilities

- `progress-logging-helper`: shared helper for logging alongside indicatif MultiProgress

### Modified Capabilities

## Non-goals

- No changes to progress bar visual style or templates
- No changes to the retry logic itself (still one retry pass)
- No changes to emukc_log or quiet_stdout behavior

## Impact

- `crates/emukc_bootstrap/src/populate.rs` — bar length fix, Arc unwrapping, active_count assert
- `crates/emukc_bootstrap/src/download.rs` — use `log_with_mp` helper, fix indentation
- `crates/emukc_bootstrap/src/progress.rs` — add `log_with_mp` function
