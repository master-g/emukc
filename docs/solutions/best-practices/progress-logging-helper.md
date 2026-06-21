---
title: "Progress/logging coexistence, aggregate-bar cap, and panic-safe failure collection"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: best_practice
component: development_workflow
severity: medium
applies_when:
  - "Implementing progress display that must coexist with tracing log output"
  - "Computing aggregate progress-bar length across retry passes"
  - "Collecting per-task failures from a concurrent download pass"
tags: [progress-bar, indicatif, multi-progress, logging, concurrency, panic-safety]
related_components: [emukc_cache]
---

# Progress/logging coexistence, aggregate-bar cap, and panic-safe failure collection

## Context

The cache populate flow renders progress bars via indicatif while also emitting
`tracing` log lines. Without a coordination mechanism, a log line written
mid-bar corrupts the bar rendering. Two further populate-specific hazards exist:
the aggregate bar can exceed 100% when a second retry pass adds items the bar
length did not account for, and failure collection used an `Arc::try_unwrap()`
that panics if the Arc is still shared.

## Guidance

**Log/progress coexistence.** The `log_with_mp` function SHALL accept an
`Option<MultiProgress>` and a closure. When `Some(mp)` is passed, it SHALL call
`mp.suspend(f)` so the closure's log output renders without colliding with the
active bars. When `None` is passed (non-TTY, no progress layer), it SHALL call
`f()` directly.

**Aggregate bar never exceeds 100%.** The aggregate progress bar in populate
SHALL have its length extended to include retry items BEFORE pass 2 begins, so
the bar position never exceeds the bar length. Concretely, when pass 1 completes
with N failures, the bar length MUST be set to `total_files + N` before pass 2
processes any item.

**Panic-safe failure collection.** The `run_pass` function SHALL collect
failures WITHOUT `Arc::try_unwrap().unwrap()`. It SHALL use a lock-based clone
(`Mutex::lock().unwrap().clone()`) instead, so that a still-shared Arc does not
panic the process.

## Why This Matters

A `log_with_mp` that ignores the `MultiProgress` produces interleaved garbage
in TTY mode. An aggregate bar whose length is not extended for retry items
visually overflows past 100%, which looks like a bug to the user and can mask
real progress. `Arc::try_unwrap().unwrap()` panics whenever the Arc has
remaining strong references — a real possibility under concurrent download —
turning a recoverable per-task failure into a process crash.

## When to Apply

- When wiring `tracing` log calls into any code path that runs under an
  indicatif `MultiProgress`.
- When computing an aggregate bar length that must cover both initial and retry
  item counts.
- When collecting results or failures from concurrent tasks that share an
  `Arc<Mutex<...>>`.

## Examples

`log_with_mp(Some(mp), || info!("downloaded X"))` suspends the bars, prints the
line, resumes. With `None` it just logs. In `run_pass`, a failed task's error is
appended via `failures.lock().unwrap().push(err.clone())`, never via
`Arc::try_unwrap`.

## Related

- `cli-progress.md` — the TTY-gated progress-bar policy these helpers implement.
