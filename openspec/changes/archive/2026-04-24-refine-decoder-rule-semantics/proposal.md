## Why

The decoder-driven `Rules` cache-list path is now functionally complete enough to cover the entire current bootstrap baseline, but it still over-generates `6382` candidate-only paths and remains dependent on Rust-authored fallback semantics for key ship and slot categories. The remaining noise is concentrated in a few ship and slot variant families, so this is the right point to tighten rule semantics before switching bootstrap to a decoder-first default.

## What Changes

- Refine decoder-emitted ship and slot rule semantics so `banner*` and `item_*2` style targets no longer expand with universal fallback behavior when the decoded runtime usage is narrower.
- Add explicit rule data for variant scope, group scope, and normalization constraints needed to distinguish friendly, abyssal, graph-driven, and runtime-normalized resource families.
- Update Rust rule execution so `CacheListMakeStrategy::Rules` consumes those narrower semantics directly instead of inferring broad variant behavior from static fallback tables.
- Reduce or eliminate the current `candidate-only` path clusters led by `ship/banner_g`, `ship/banner2_g`, `ship/banner3_g`, `ship/banner_dmg`, `ship/banner_g_dmg`, `slot/item_on2`, and `slot/item_up2`.
- Move decoder rule generation further toward a single-source-of-truth model by reducing reliance on Rust constants mirrored back into decoder outputs.

## Non-goals

- Do not switch the default bootstrap or CLI cache-list strategy from `Default`/`Manifest` to `Rules` in this change.
- Do not redesign unrelated cache domains that already have stable parity, such as BGM, map, furniture, sound, useitem, or voice.
- Do not remove existing legacy generation code paths yet; they remain fallback and validation tools during this phase.

## Capabilities

### New Capabilities

- `decoder-rule-semantics`: define decoder-emitted rule semantics for ship and slot resource generation scope, especially for damage variants, group-scoped targets, and runtime normalization families.

### Modified Capabilities

- `cache-manifest-integration`: change decoder-driven cache-list generation requirements so the `Rules` path must prefer decoder-authored semantic constraints over broad fallback expansion when producing ship and slot paths.
- `decoder-coverage-assets`: change decoder coverage asset requirements so rule outputs can encode precise ship/slot generation semantics without copying Rust-authored path constants back into decoder assets.

## Impact

- Affected code in `main-decoder/src/cache-rules.ts`, `main-decoder/src/resource-manifest.ts`, `main-decoder/src/resource-categories.ts`, and `main-decoder/src/path-rules.ts`.
- Affected Rust execution in `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` and related manifest/rule loading paths.
- Affected validation workflow in `examples/decoder_cachelist_compare.rs` and the generated `.data/decoder_rules_compare*.json` reports.
- No gameplay trait, database schema, or KCSAPI route-group behavior changes are expected in this change.
