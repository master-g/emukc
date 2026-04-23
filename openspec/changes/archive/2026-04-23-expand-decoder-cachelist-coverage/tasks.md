## 1. Decoder Coverage Assets

- [x] 1.1 Extend `main-decoder/src/types.ts`, `main-decoder/src/pipeline.ts`, and `main-decoder/src/cli.ts` to model the decoder coverage asset bundle and write it to `main-decoder/out/resources/` with optional bootstrap sync support
- [x] 1.2 Implement sparse ship/slot subset extraction in `main-decoder/src/resource-id-sets.ts`, including provenance and explicit `coverageMode` handling, and add focused tests
- [x] 1.3 Implement `main-decoder/src/audio-resources.ts` for SE, categorized BGM, and voice/titlecall/tutorial coverage, and add focused tests
- [x] 1.4 Implement `main-decoder/src/ui-resources.ts` for map, furniture, useitem, area, and world-select coverage, and add focused tests

## 2. Comparison Loop Updates

- [x] 2.1 Update `examples/decoder_cachelist_compare.rs` so a manifest path under decoder output also loads sibling decoder coverage assets from the same output root
- [x] 2.2 Extend the comparison report to include domain-level coverage metrics and grouped sparse-category deltas that highlight the highest-impact over/under-generation buckets

## 3. Rust Coverage Asset Loading

- [x] 3.1 Add Rust serde types and loaders for `resource_categories.json`, `resource_id_sets.json`, `audio_resources.json`, and `ui_resources.json` under `crates/emukc_bootstrap/src/make_list/`
- [x] 3.2 Thread manifest-adjacent decoder asset overrides through the manifest generation path so candidate generation can consume decoder outputs directly without repo-asset sync

## 4. Decoder-Driven Cache List Integration

- [x] 4.1 Use decoder category groups to fill deterministic ship/slot gaps such as `power_up` and `card_t` in the decoder-driven cache-list path
- [x] 4.2 Use sparse decoder subset assets to constrain sparse ship/slot categories such as `special`, `sp_remodel/*`, `card_round`, and `reward_*` instead of expanding them universally
- [x] 4.3 Extend decoder-driven generation to consume audio coverage assets for sound, BGM, and voice domains while preserving fallback behavior for unresolved categories
- [x] 4.4 Extend decoder-driven generation to consume UI coverage assets for map, furniture, useitem, area, and world-select domains while preserving fallback behavior for unresolved categories

## 5. Verification

- [x] 5.1 Run `cd main-decoder && bun test` and `cargo test -p emukc_bootstrap` after the decoder and Rust integration changes
- [x] 5.2 Run the decoder to emit the full coverage asset bundle, then run `cargo run --example decoder_cachelist_compare ...` to verify the new domain-level report and confirm that decoder-driven cache-list coverage improves versus the current baseline
- [x] 5.3 Run `cargo clippy --workspace` after integration to confirm the new decoder coverage path does not introduce new workspace warnings
