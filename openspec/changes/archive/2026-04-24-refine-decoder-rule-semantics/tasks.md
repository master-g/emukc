## 1. Decoder semantic rule modeling

- [x] 1.1 Audit the current `banner*` and `item_*2` extraction paths in `main-decoder/src/cache-rules.ts`, `main-decoder/src/resource-manifest.ts`, and `main-decoder/src/resource-categories.ts` and define the semantic fields needed for canonical ship/slot rule output.
- [x] 1.2 Update decoder ship rule extraction so `cache_rules.json` encodes canonical ship variant scope for `banner`, `banner_g`, `banner2_g`, `banner3_g`, and related families, including selector scope and completeness state.
- [x] 1.3 Update decoder slot rule extraction so `cache_rules.json` encodes normalization-scoped alias behavior for `item_on2` and `item_up2`, including unresolved markers when runtime evidence is incomplete.
- [x] 1.4 Reduce `main-decoder/src/path-rules.ts` dependence on Rust-authored constants for ship/slot semantic truth while preserving any parity-only fallback data still needed in this phase.
- [x] 1.5 Add or update Bun tests that assert the emitted semantic rule bundle for the targeted ship and slot families.

## 2. Rust rule execution updates

- [x] 2.1 Extend the bootstrap manifest/rule types and loaders in `crates/emukc_bootstrap/src/make_list/manifest/` so Rust can parse the new ship/slot semantic fields from `cache_rules.json`.
- [x] 2.2 Update `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` so ship path generation applies decoder semantic scope before consulting legacy damage-variant fallback tables.
- [x] 2.3 Update `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` and `crates/emukc_bootstrap/src/make_list/manifest/resolve.rs` so slot alias families such as `item_on2` and `item_up2` use decoder normalization semantics before universal slot expansion.
- [x] 2.4 Add Rust unit tests covering damaged-only ship targets, constrained variant families, normalized slot alias targets, and unresolved fallback behavior.

## 3. Regression validation

- [x] 3.1 Regenerate the decoder resource bundle so `main-decoder/out/resources/cache_rules.json` and sibling assets reflect the new semantic rules.
- [x] 3.2 Run `cargo run --example decoder_cachelist_compare -- --config emukc.config.toml --rules main-decoder/out/resources/cache_rules.json --report-json .data/decoder_rules_compare_current.json` and capture the updated overlap report.
- [x] 3.3 Verify that baseline coverage remains `100%` while the targeted `candidate-only` ship/slot prefixes are materially reduced, and document the before/after numbers in the change notes or commit message.
