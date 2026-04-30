## Why

CLI subcommands that use `indicatif` progress bars (`cache populate`, `bootstrap`) output tracing log lines to stdout while progress bars render to stderr. This produces garbled terminal output — log lines and progress bars interleave unpredictably. Logs should still be written to file for debugging, but stdout must be silenced when TUI progress bars are active.

## What Changes

- Add `with_quiet_stdout(bool)` method to `emukc_log::Builder` that suppresses the stdout fmt layer during `build()`
- When `quiet_stdout` is true, `build()` registers only the file appender layer (no stdout layer)
- CLI entry point (`cli/mod.rs`) detects subcommands that use indicatif progress bars and sets `with_quiet_stdout(true)` on the log builder

## Capabilities

### New Capabilities

- `quiet-stdout`: Ability to suppress stdout log output while retaining file logging, used when TUI progress bars are active

### Modified Capabilities

_(none)_

## Non-goals

- Runtime log suppression (switching stdout on/off mid-execution) — not needed; the decision is made once at startup
- Changing indicatif's output target (currently stderr) — works fine, no reason to change
- Modifying the `MultiProgress::suspend()` pattern already in `download.rs` — that pattern remains valid for non-TTY or edge cases

## Impact

- `crates/emukc_log/src/log.rs` — new field + setter + conditional in `build()`
- `src/bin/cli/mod.rs` — conditional `with_quiet_stdout()` call based on subcommand
- No API changes, no dependency additions, no breaking changes
