## 1. Decoder Bundle Plumbing

- [x] 1.1 Add decoder-first bundle/context types and loader helpers in `crates/emukc_bootstrap/src/make_list/manifest/` so a `cache_rules.json` path can also resolve sibling decoder coverage assets from the same output tree.
- [x] 1.2 Thread the decoder-first bundle through `crates/emukc_bootstrap/src/make_list/mod.rs` and `crates/emukc_bootstrap/src/make_list/source/mod.rs` so the `Rules` path can use explicit bundle inputs without silently falling back to repo assets first.
- [x] 1.3 Extend loader tests in `crates/emukc_bootstrap/src/make_list/` to cover explicit rules-path bundle loading, repo-asset fallback, and missing optional sibling assets.

## 2. Rule-First Generation and Attribution

- [x] 2.1 Refactor `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` and related helpers so decoder-first generation runs in explicit rule-authoritative and fallback-fill phases.
- [x] 2.2 Add sideband authority accounting in `crates/emukc_bootstrap/src/make_list/` that records rule-authored paths, fallback-authored paths, and grouped residual family keys without changing `CacheListItem` serialization.
- [x] 2.3 Update rules-driven resource extension code in `crates/emukc_bootstrap/src/make_list/source/kcs2/` so sibling decoder audio/UI/category assets are applied in the `Rules` path, not only the `Manifest` path.
- [x] 2.4 Add Rust tests that prove decoder-covered families suppress broad fallback expansion while partial or unresolved families still generate via attributable fallback.

## 3. Comparison and Migration Diagnostics

- [x] 3.1 Extend `examples/decoder_cachelist_compare.rs` and `crates/emukc_bootstrap/src/make_list/mod.rs` so `--rules` runs consume the full decoder bundle instead of `cache_rules.json` alone.
- [x] 3.2 Add comparison report fields and CLI output for rule-authored counts, fallback-authored counts, fallback residual prefixes/families, and unresolved-rule blockers.
- [x] 3.3 Add or update tests for comparison reporting so decoder-first candidate runs expose migration blockers clearly and report zero fallback residuals when authority is complete.

## 4. Decoder Output and Verification

- [x] 4.1 Update `main-decoder/src/types.ts`, `main-decoder/src/cache-rules.ts`, and `main-decoder/src/pipeline.ts` only as needed to support stable decoder-first bundle metadata and residual reporting consumed by Rust.
- [x] 4.2 Run `cd main-decoder && bun test` plus targeted Rust tests for `emukc_bootstrap` after the bundle and reporting changes.
- [x] 4.3 Regenerate the decoder outputs and run `cargo run --example decoder_cachelist_compare -- --config emukc.config.toml --rules main-decoder/out/resources/cache_rules.json --report-json .data/decoder_rules_compare_current.json` to capture the updated authority/fallback report for this change.
