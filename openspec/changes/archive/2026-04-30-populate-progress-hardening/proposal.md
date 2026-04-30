## Why

`cache populate` has three UX problems: (1) failed task spinners remain on screen as permanent lines instead of being cleared, cluttering the terminal; (2) the first download error immediately terminates the entire populate run, abandoning all in-flight tasks; (3) there is no end-of-run summary showing what succeeded, what failed after retry, or why.

## What Changes

- Spinner behavior on error: clear the spinner line instead of leaving `✗ path (error)` visible. Collect error details into an in-memory queue for later display.
- Error handling: stop propagating the first error via `result?`. Instead, collect all failures, let all tasks complete, then retry the failed items once. Only report failure if items still fail after retry.
- End-of-run summary: print a structured summary after all work completes, showing total/OK/failed counts, retry outcomes, elapsed time, and a list of files that ultimately failed with their error reasons.

## Capabilities

### New Capabilities
- `populate-retry`: retry logic for failed populate tasks — collect failures, retry once, report final results

### Modified Capabilities
- `populate-progress`: spinner error behavior changes from `finish_with_message` to `finish_and_clear` with error collection

## Non-goals

- Changing download.rs error handling or progress display (separate concern)
- Adding configurable retry count (hardcoded single retry is sufficient)
- Persistent retry state across process restarts

## Impact

- `crates/emukc_bootstrap/src/populate.rs` — main logic changes
- `crates/emukc_bootstrap/src/progress.rs` — possible helper additions for summary formatting
