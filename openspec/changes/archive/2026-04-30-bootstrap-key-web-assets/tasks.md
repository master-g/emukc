## 1. CLI Flag

- [x] 1.1 Add `--skip-web-assets` flag to bootstrap CLI command in `src/bin/cli/bootstrap.rs`

## 2. Core Download Function

- [x] 2.1 Add `download_web_assets()` function in `crates/emukc_bootstrap/src/` that downloads kcs_const.js, main.js, and version.json via Kache
- [x] 2.2 Handle missing CDN config: skip download and emit warn-level log with config guidance
- [x] 2.3 Handle existing files: skip when overwrite=false, emit debug log

## 3. Bootstrap Integration

- [x] 3.1 Call `download_web_assets()` in bootstrap flow after Codex build, gated by `--skip-web-assets` flag
- [x] 3.2 Integrate with `--force-update`: delete old web assets then re-download

## 4. Testing

- [x] 4.1 Verify `cargo build` compiles with new code
- [x] 4.2 Run `cargo test` and ensure no regressions
