## Context

The `indicatif-progress-bars` change added progress bars to `populate.rs`, `download.rs`, and `make_list/progress.rs`. Code review found two bugs and a maintenance issue before merge.

Current state:
- `populate.rs` has a 2-pass retry strategy sharing a single `aggregate_pb` bar
- `run_pass` collects failures via `Arc<std::sync::Mutex<Vec<FailedItem>>>` and unwraps with `Arc::try_unwrap().unwrap().into_inner().unwrap()`
- `download.rs` has ~8 instances of the `if let Some(mp) { mp.suspend(|| { log!() }) } else { log!() }` pattern
- Some new code in download.rs uses spaces instead of hard tabs

## Goals / Non-Goals

**Goals:**
- Fix aggregate bar overflow in pass 2
- Harden failure collection against panics
- Eliminate log/suspend duplication
- Fix indentation

**Non-Goals:**
- No progress bar style changes
- No retry logic changes
- No logging pipeline changes

## Decisions

### 1. Aggregate bar: extend length before pass 2

Use `pb.set_length(total_files + retry_count)` before pass 2. The bar position is cumulative (pass 1 + pass 2 increments), so the length must match total work.

Alternative: `pb.reset()` + new length of `retry_count`. Rejected — bar would show 0→N retry items, losing the visual context that N items already completed.

### 2. Failure collection: clone under lock instead of try_unwrap

Replace `Arc::try_unwrap(failures).unwrap().into_inner().unwrap()` with `failures.lock().unwrap().clone()`. The Vec is small (only failed items), so the clone cost is negligible. Removes two panic surfaces.

### 3. Log helper: `log_with_mp` in progress.rs

Add `pub fn log_with_mp(mp: &Option<MultiProgress>, f: impl FnOnce())` that calls `mp.suspend(f)` or `f()` directly. Replace all 8 instances in download.rs.

### 4. Active count: debug_assert between passes

Add `debug_assert_eq!(active_count.load(Ordering::Relaxed), 0)` after each `run_pass` call. Documents the invariant, catches logic errors in debug builds, zero cost in release.

## Risks / Trade-offs

- **Bar length "stretches"** after pass 1 completes → bar resets from 100% to ~95% visually. Acceptable: the message says "retry N items" so the user knows work continues. Alternative of separate bar adds complexity for minimal gain.
