## 1. Data Structures

- [x] 1.1 Add `FailedItem` struct to `populate.rs` with fields: `path: String`, `version: Option<String>`, `error: String`
- [x] 1.2 Add `PopulateStats` struct to track: total, succeeded, retried, recovered, failed counts and elapsed time

## 2. Spinner Error Behavior

- [x] 2.1 In `populate.rs` task closure error branch: change `sp.finish_with_message(...)` to `sp.finish_and_clear()`
- [x] 2.2 Add `Arc<Mutex<Vec<FailedItem>>>` to populate function for collecting failures across concurrent tasks
- [x] 2.3 In the error branch, push `FailedItem` into the failure collection instead of propagating error

## 3. Error Collection (Remove Early Termination)

- [x] 3.1 Change task closure return type to not require `?` propagation — return `Result<(), FailedItem>` from each task
- [x] 3.2 Replace `result?` at the outer loop with result collection: match `Ok(())` silently, collect `Err(failed_item)` into the failure vec
- [x] 3.3 Keep fatal IO errors (list file read/parse) as early returns — only task-level errors are collected

## 4. Retry Pass

- [x] 4.1 After pass 1 completes, drain the failure vec into a retry queue
- [x] 4.2 Run a second pass over the retry queue using the same concurrency + progress bar logic (reuse `FuturesUnordered` loop)
- [x] 4.3 Track retry outcomes: successes = recovered, failures = final failures

## 5. Summary

- [x] 5.1 Add `print_summary()` function to `progress.rs` that formats `PopulateStats` + final failure list
- [x] 5.2 In TTY mode: use `MultiProgress::suspend` to print summary above finished bars
- [x] 5.3 In non-TTY mode: print summary directly to stdout via `println!`
- [x] 5.4 Call `print_summary()` after all passes complete, before function return

## 6. Return Value

- [x] 6.1 If final_failures is empty → `Ok(())`
- [x] 6.2 If final_failures is non-empty → `Err(KacheError::...)` with a concise summary message

## 7. Verification

- [x] 7.1 `cargo build` — no compile errors
- [x] 7.2 `cargo clippy -p emukc_bootstrap` — no new warnings
- [x] 7.3 Manual test: run `cache populate` and verify spinner cleanup, retry behavior, and summary output
