## Context

`emukc_log::Builder` currently always registers a stdout `fmt::layer()` in `build()`. When CLI subcommands use `indicatif` progress bars (rendered to stderr), tracing log lines still emit to stdout, causing garbled terminal output.

The logging architecture:
- `build()` registers: stdout fmt layer + optional file appender layer
- `build_simple()` registers: stdout fmt layer only
- Both share the same `EnvFilter`

Affected subcommands: `bootstrap`, `cache populate` (and future commands using indicatif).

## Goals / Non-Goals

**Goals:**
- Suppress stdout log output when progress bars are active
- Retain file logging regardless of stdout suppression
- Keep the change minimal — single field on Builder, no new types

**Non-Goals:**
- Runtime log toggling (stdout on/off mid-execution)
- Changing indicatif's stderr target
- Removing the `MultiProgress::suspend()` pattern in `download.rs`

## Decisions

### Decision 1: Add `quiet_stdout: bool` field to Builder

Set at build time, checked in `build()` and `build_simple()`.

**Why not runtime writer switching?** tracing-subscriber doesn't support swapping layers after registration. A `Box<dyn Write>` wrapper would add complexity for no benefit — the quiet decision is known at startup.

**Why not TTY auto-detection in Builder?** Builder lives in `emukc_log` which has no concept of CLI subcommands. The caller (`cli/mod.rs`) knows whether the current subcommand uses indicatif, so the decision belongs there.

### Decision 2: Caller determines quiet mode

`cli/mod.rs::init()` checks the subcommand variant and passes `with_quiet_stdout(true)` before `build()`.

Subcommands that SHALL use quiet stdout:
- `Bootstrap` — uses `MultiProgress` + progress bars in `download.rs`
- `Cache(Populate)` — uses `ProgressBar` in `populate.rs`

Other subcommands keep normal stdout logging.

### Decision 3: File layer always registered

When `quiet_stdout` is true and file appender is configured, only the file layer is registered. If no file appender is configured and `quiet_stdout` is true, no layers are registered (logs go nowhere — acceptable for CI/automation).

## Risks / Trade-offs

- **[Risk] No stdout logs during bootstrap/populate** → Acceptable: file logs capture everything. User can check file if something goes wrong. `--log none` already exists as precedent for suppressing output.
- **[Risk] Forgetting to add `with_quiet_stdout` for future indicatif-using commands** → Low impact: worst case is garbled output, not broken functionality. Easy to fix.
