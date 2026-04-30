## Context

EmuKC's CLI long-running operations (`cache populate`, `bootstrap`, `cache make-list`) use two output mechanisms:
1. `tracing` info/debug/warn log lines (via `emukc_log`)
2. Hand-rolled `\r` progress counters (`print_progress()` in `populate.rs`, `ProgressTracker` in `make_list/progress.rs`)

These two streams clobber each other — log lines break the in-place progress update, and the progress counter provides no speed or ETA information.

The project uses `tracing` for all structured logging, initialized in `emukc_log::Builder` with stdout + optional file appender. The `emukc_bootstrap` crate contains all affected code.

Current output patterns:
- `populate.rs`: `[completed/total][percentage%]` via `\r` overwrite, no speed/ETA
- `download.rs`: `info!()` per resource with size/md5/time — no aggregate progress
- `make_list/progress.rs`: `info!("Progress: checked/found/rate/ETA")` — periodic log lines, no visual bar
- `bootstrap.rs`: interleaved info/warn logs, no phase structure

## Goals / Non-Goals

**Goals:**
- Structured progress bars with percentage, speed, and ETA for all long-running CLI operations
- Phase-labeled output for multi-step operations (bootstrap has 4 phases)
- Log lines printed cleanly without clobbering active progress bars
- Non-TTY fallback: degrade gracefully to plain log output

**Non-Goals:**
- No full TUI framework (ratatui)
- No `tracing-indicatif` span-based integration (concurrent task model doesn't fit)
- No changes to `emukc_log` tracing subscriber pipeline
- No interactive controls (pause/resume/cancel)
- No changes to server-mode output

## Decisions

### Decision 1: Use `indicatif` directly, not `tracing-indicatif`

**Rationale**: `tracing-indicatif` maps one span → one progress bar. But `populate` uses `FuturesUnordered` with `AtomicUsize` counter — N concurrent tasks share one aggregate bar. Span-per-task would create N bars that appear/disappear rapidly. Direct `indicatif` gives precise control.

**Alternative considered**: `tracing-indicatif` layer — rejected due to mismatch with concurrent download model.

### Decision 2: `MultiProgress` for concurrent download visibility

`download.rs` runs up to 4 concurrent downloads. Use `MultiProgress` to show:
- One aggregate progress bar (resource N/M)
- Per-resource spinners showing current download names

`MultiProgress::suspend()` wraps any `info!/warn!` calls to prevent clobbering.

### Decision 3: `ProgressBar` style templates

```
populate:  "Populating cache  ━━━━━━━━━━━  {percent}% {eta}"
bootstrap: "Phase 1/4: Resources  ━━━━━━━  {pos}/{len} {eta}"
make-list: "Checking resources  ━━━━━━━━━  {percent}% {msg}"
```

Use `ProgressStyle::with_template()` for consistency. All bars write to stderr.

### Decision 4: Non-TTY detection

Check `std::io::stdout().is_terminal()` (via `is-terminal` crate, already indirect dep). If not a TTY, skip progress bars and let existing `info!()` logs serve as output — no behavior change from current state.

### Decision 5: Dependency scope

Add `indicatif` to workspace `Cargo.toml`, import only in `emukc_bootstrap`. No changes to `emukc_log` crate — progress bars are a presentation concern of the bootstrap module, not the logging infrastructure.

## Risks / Trade-offs

- **[Non-TTY environments]** `indicatif` draws to stderr. In CI/piped contexts, bars may produce garbage → Mitigation: check `is_terminal()` before creating bars, fall back to log-only
- **[Log clobbering]** `tracing` writes to stdout, `indicatif` writes to stderr → they can still interleave badly → Mitigation: wrap log emissions in `MultiProgress::suspend()` or redirect tracing to stderr via `indicatif` writer
- **[Dependency addition]** `indicatif` adds ~50KB compiled → acceptable for a CLI tool
- **[Thread safety]** `ProgressBar::inc()` is thread-safe (uses `AtomicU64` internally) — no mutex needed for concurrent tasks
