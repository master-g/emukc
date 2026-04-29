## 1. Decoder Sound Rule Extraction

- [x] 1.1 Extend `main-decoder/src/types.ts` and `main-decoder/src/cache-rules.ts` so `cache_rules.json` can carry sound-rule metadata for algorithmic `kcs/sound/*` families.
- [x] 1.2 Implement decoder extraction for formula-driven ship voice rules, including `api_voicef`-gated voice families and special ship voice subsets currently modeled only in Rust.
- [x] 1.3 Implement decoder extraction for bucket-driven sound families such as `kc9997`, `kc9998`, and `kc9999`, including random-choice voice groups where decoded modules expose them.
- [x] 1.4 Add or update Bun tests that assert the emitted sound-rule bundle for ship voice formulas and `9997` / `9998` / `9999` families.

## 2. Rust Rules-Path Sound Migration

- [x] 2.1 Extend Rust cache-rule serde/loading in `crates/emukc_bootstrap/src/make_list/manifest/` so the `Rules` path can parse decoder-authored sound rules.
- [x] 2.2 Update `crates/emukc_bootstrap/src/make_list/source/kcs/voice.rs` so covered ship voice families are generated from decoder sound rules before consulting the existing Rust fallback formulas and tables.
- [x] 2.3 Update `crates/emukc_bootstrap/src/make_list/source/kcs/kc9997.rs`, `kc9998.rs`, and `kc9999.rs` so covered bucket families are generated from decoder sound rules first, with unresolved portions preserved as explicit fallback.
- [x] 2.4 Add Rust tests that prove covered sound families are rule-authored while unresolved sound families remain attributable fallback.

## 3. Comparison and Validation

- [x] 3.1 Extend `examples/decoder_cachelist_compare.rs` and related report structures so sound-domain fallback residuals and sound-rule migration progress are visible in the compare output.
- [x] 3.2 Regenerate the decoder bundle and run `cargo run --example decoder_cachelist_compare -- --config emukc.config.toml --rules main-decoder/out/resources/cache_rules.json --report-json .data/decoder_rules_compare_current.json`.
- [x] 3.3 Verify that baseline recall remains `100%` and that the `kcs/sound/*` fallback-authored residual drops materially from the current baseline before closing the change.
