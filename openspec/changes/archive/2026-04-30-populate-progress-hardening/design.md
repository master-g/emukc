## Context

`populate()` in `crates/emukc_bootstrap/src/populate.rs` fetches cache items concurrently using `FuturesUnordered`. Current behavior: per-task spinners persist on error via `finish_with_message`, first error kills the entire run via `result?`, and the only "summary" is a progress bar finish message.

The function uses `MultiProgress` from indicatif with three visual elements: an aggregate progress bar, a stats bar, and per-task spinners.

## Goals / Non-Goals

**Goals:**
- Clear error spinners instead of leaving permanent lines
- Collect failures and retry once before giving up
- Print a structured summary at the end of the run

**Non-Goals:**
- Configurable retry count
- Persistent retry state across process restarts
- Changes to `download.rs` or `make_list/progress.rs`

## Decisions

### D1: Error spinner behavior — `finish_and_clear` instead of `finish_with_message`

Change `populate.rs:107` from `sp.finish_with_message(...)` to `sp.finish_and_clear()`. Store `(path, error_string)` in a `Mutex<Vec<FailedItem>>` for later display.

Rationale: keeping error spinners visible adds terminal noise. Errors are better surfaced in the final summary where they're complete and deduplicated.

### D2: Failure collection via `Arc<Mutex<Vec<FailedItem>>>` instead of `result?`

Replace `result?` at `populate.rs:132` with error collection. The task closure returns `Result<(), FailedItem>` — `Ok(())` on success, `Err(FailedItem)` on failure. The outer loop collects results without short-circuiting.

```
struct FailedItem {
    path: String,
    error: String,
}
```

Rationale: `Arc<Mutex<Vec>>` is the standard pattern for collecting errors across concurrent tasks. The Mutex is uncontended in practice (errors are rare relative to total items).

### D3: Two-pass execution

After the first pass completes all items, check the failure queue. If non-empty, run a second pass over just those items. Items that fail twice go into the final failure list.

```
Pass 1: all items → successes + failures_1
Pass 2: failures_1 → recovered + final_failures
```

Rationale: transient network errors are the most common failure mode. A single retry catches most of them without added complexity.

### D4: Summary printed via `MultiProgress::suspend`

After all passes complete, use `mp.suspend(|| print_summary(...))` to print the summary above the (now-finished) progress bars. This avoids visual conflicts with indicatif's rendering.

Summary format:
```
── Populate Summary ─────────────────────
 Total: 1000 │ OK: 987 │ Retried: 13 │ Recovered: 11 │ Failed: 2
 Time: 2m 34s

 Failed files:
   ✗ kcs2/sound/bgm/123.mp3 (timeout)
   ✗ kcs2/resource/ship/456.png (checksum mismatch)
```

### D5: Return `Result` based on final failures

If `final_failures` is empty → `Ok(())`. If non-empty → `Err(KacheError::...)` with a summary message. The CLI layer can decide how to handle this (exit code, etc.).

## Risks / Trade-offs

- **Mutex contention on error path**: negligible — errors are rare relative to total items, and the critical section is a single `push`.
- **Second pass re-reads lines**: the list file is read line-by-line in pass 1. For pass 2, we already have `FailedItem` structs with the path and item data. We can reconstruct `CacheListItem` from stored data or re-read. Storing the full `CacheListItem` in `FailedItem` avoids re-reading the file.
- **Summary in non-TTY mode**: `MultiProgress::suspend` is a no-op when `mp` is `None`. Need a separate non-TTY summary path that prints to stdout.
