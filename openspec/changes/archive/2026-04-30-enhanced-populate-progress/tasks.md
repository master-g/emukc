## 1. Progress Infrastructure

- [x] 1.1 Add `new_stats_bar(concurrency: usize)` helper to `progress.rs` — returns `Option<ProgressBar>` with `"{msg}"` template, `enable_steady_tick` for periodic refresh, initial message set to `"0/{concurrency} active │ 0 errors"`
- [x] 1.2 Add `update_stats_message(pb: &ProgressBar, active: usize, max_concurrent: usize, errors: usize)` helper that sets the stats bar message

## 2. Populate Refactor

- [x] 2.1 In `populate()`, create `Arc<AtomicUsize>` for `active_count` and `error_count`
- [x] 2.2 Create `MultiProgress` via `new_multi_progress()`, add aggregate bar and stats bar to it
- [x] 2.3 Before pushing each task to `FuturesUnordered`, create a per-task spinner via `new_spinner(&item.path)` and add it to `MultiProgress`
- [x] 2.4 Inside each task: increment `active_count` on start, call `opt.get()`, on success `spinner.finish_and_clear()`, on error `spinner.finish_with_message(format!("✗ {path} ({err})"))` and increment `error_count`, always decrement `active_count`
- [x] 2.5 Update stats bar message after each task completion using the atomic counters

## 3. Finalization

- [x] 3.1 After all tasks complete, finish aggregate bar with message including total error count: `format!("Populating cache  done ({total} files, {errors} errors)")`
- [x] 3.2 Clear stats bar with `finish_and_clear()`
- [x] 3.3 Verify non-TTY path: pipe output, confirm no progress bars appear and no ANSI codes in output

## 4. Build Verification

- [x] 4.1 `cargo build` — no compile errors
- [x] 4.2 `cargo clippy --workspace` — no new warnings
