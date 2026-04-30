## 1. emukc_log Builder Changes

- [x] 1.1 Add `quiet_stdout: bool` field to `Builder` struct in `crates/emukc_log/src/log.rs`, default `false`
- [x] 1.2 Add `pub fn with_quiet_stdout(mut self, quiet: bool) -> Self` setter method
- [x] 1.3 Modify `build()` — when `quiet_stdout` is true, skip registering the stdout fmt layer; register file layer only (or nothing if no file appender)
- [x] 1.4 Modify `build_simple()` — when `quiet_stdout` is true, register a no-op collector (or skip stdout layer)

## 2. CLI Integration

- [x] 2.1 Add helper function `fn needs_quiet_stdout(cmd: &Commands) -> bool` in `src/bin/cli/mod.rs` returning true for `Bootstrap` and `Cache(CacheArgs)` when inner command is `Populate`
- [x] 2.2 In `init()`, call `builder.with_quiet_stdout(needs_quiet_stdout(&args.command))` before `.build()`

## 3. Verification

- [x] 3.1 `cargo build` — no compile errors
- [x] 3.2 `cargo clippy --workspace` — no new warnings
