## Context

Cache populate (`crates/emukc_bootstrap/src/populate.rs`) runs concurrent downloads using `FuturesUnordered`. stdout is suppressed via `with_quiet_stdout(true)` in the CLI. A single `indicatif` progress bar shows file count and ETA. No visibility into concurrency, active downloads, or errors.

The existing `progress.rs` module already provides `MultiProgress` helpers used by `download.rs`. The populate path does not yet use `MultiProgress`.

## Goals / Non-Goals

**Goals:**
- Show active concurrent task count vs configured concurrency
- Show each in-flight download's resource path via individual spinners
- Track and display error count; briefly show error details in the spinner before clearing
- Use existing `indicatif` + `MultiProgress` infrastructure

**Non-Goals:**
- Download speed / bytes-per-second tracking (not visible at populate layer)
- Full-screen TUI with ratatui
- Changes to `emukc_cache` API or download internals
- Spinner count capping (show one spinner per concurrent task; user controls concurrency via CLI flag)

## Decisions

### 1. MultiProgress layout: aggregate bar + stats bar + per-task spinners

Three tiers in the `MultiProgress`:

```
[aggregate bar]  Populating cache  ━━━━━━━━━━  1234/5678 (21%, ETA: 04:32)
[stats bar]      16/32 active  │  3 errors
[spinner 1]      ⠋ kcs2/resources/ship/banner/001.png
[spinner 2]      ⠋ kcs2/resources/bgm/123.ogg
...
[spinner N]      ⠋ kcs2/resources/ship/full/003.png
```

**Rationale**: `MultiProgress` already used in `download.rs`. Reusing it keeps the pattern consistent. Stats bar as a separate `ProgressBar` (style `ProgressStyle::with_template("{msg}")`) avoids cramming stats into the aggregate bar template.

### 2. Atomic counters for stats

`AtomicUsize` for `active_count` and `error_count`, wrapped in `Arc`. Each task increments/decrements on start/finish. Stats bar updates via `enable_steady_tick` to poll atomics periodically.

**Rationale**: No mutex needed for simple counters. `enable_steady_tick` on the stats bar avoids manual refresh logic.

### 3. Spinner lifecycle tied to task lifecycle

Create spinner before pushing task to `FuturesUnordered`. Inside the task:
- On success: `spinner.finish_and_clear()`
- On error: `spinner.finish_with_message(format!("✗ {path} ({err})"))`, then the spinner stays visible briefly showing the error before the next cycle cleans it up.

Wait for a short delay (e.g., 2s) or just let the error spinner persist until the bar completes — simpler approach is `finish_with_message` which keeps the error line visible.

**Rationale**: Error spinners staying visible gives the user time to read what failed. Final summary from the aggregate bar still shows total error count.

### 4. No changes to cache layer

`opt.get()` return type unchanged. Error info comes from the `Result` at the populate level. Resource path comes from `item.path` which is the relative path (e.g., `kcs2/resources/ship/banner/001.png`).

**Rationale**: Avoiding cache layer changes keeps scope small. The resource path is sufficient context for the user.

## Risks / Trade-offs

- **[Many spinners with high concurrency]** → With concurrent=32, 32 spinners fill the terminal. Acceptable since user chose that concurrency; can reduce via CLI flag.
- **[Error spinners accumulate]** → Error spinners persist, reducing vertical space for active downloads. Mitigated by error count in stats bar; user can see total without reading every error line.
- **[Non-TTY fallback]** → Existing `is_tty()` check returns `None` for all bars. Non-TTY path unchanged, logs go to file as before.
