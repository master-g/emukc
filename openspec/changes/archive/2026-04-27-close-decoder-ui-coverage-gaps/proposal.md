## Why

The decoder-first cache-list pipeline already reproduces the baseline set, but UI-heavy families are still primarily fallback-authored because `main-decoder` emits empty or near-empty UI coverage assets for map, useitem, area, and world-select resources. Closing this focused gap is the next safe step toward improving cache-list ownership without changing bootstrap defaults.

## What Changes

- Strengthen `main-decoder` UI resource extraction so live decoded modules emit concrete map, useitem card/card_ IDs, area resource IDs, world-select files, and more useful furniture explicit paths when those members are observable.
- Keep UI coverage assets evidence-based: unresolved runtime-driven groups remain partial or unresolved instead of being backfilled from Rust fallback constants.
- Update Rules-path cache-list generation so decoder-provided UI members are emitted as rule-authored output first, while legacy fallback remains responsible only for uncovered residual members.
- Expand verification around the decoder UI assets and `decoder_cachelist_compare` fallback attribution so progress is measured by shrinking targeted fallback-authored prefixes without losing baseline recall.

## Non-goals

- Do not switch the default bootstrap CLI or cache-list strategy to `CacheListMakeStrategy::Rules`.
- Do not broaden this change into `kcs/sound/*` bucket rules, BGM extraction, or ship/slot precision cleanup.
- Do not remove Rust fallback generators wholesale; fallback remains required for incomplete UI families.
- Do not change gameplay traits such as `SortieOps`, `MaterialOps`, or `ShipOps`.
- Do not change KCSAPI route groups under `src/bin/net/router/kcsapi/`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `decoder-coverage-assets`: require UI coverage assets to emit concrete decoder-observable members for migration-critical map, furniture, useitem, area, and world-select families instead of leaving those groups empty.
- `cache-manifest-integration`: require Rules-path cache-list generation to attribute decoder-covered UI members as rule-authored output and keep fallback attribution limited to uncovered residual members.

## Impact

- `main-decoder/src/ui-resources.ts`, `main-decoder/src/pipeline.ts`, and `main-decoder/test/` UI coverage tests.
- Decoder output and synced bootstrap assets: `main-decoder/out/resources/ui_resources.json` and `crates/emukc_bootstrap/assets/ui_resources.json`.
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/*.rs` where UI coverage assets are consumed and fallback attribution is applied.
- `examples/decoder_cachelist_compare.rs` reports under `.data/` used to verify fallback-authored UI prefix reduction.
