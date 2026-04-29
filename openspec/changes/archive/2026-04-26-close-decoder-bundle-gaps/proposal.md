## Why

The decoder-first cache-list pipeline now has the right overall shape, but the emitted decoder bundle is still too sparse in the families that dominate fallback. The current comparison report reaches `100%` baseline recall with zero unresolved rule blockers, yet still reports `8335` fallback-authored candidate paths because the decoder output for high-value sound and UI families is still mostly empty or partial.

That leaves the project in an awkward middle state: pipeline plumbing is ready, but `main-decoder` is not yet producing strong enough bundle data to let `Rules` generation retire the biggest Rust-owned lists. This is the right next change because it turns the remaining migration work into a targeted asset-closure problem instead of more generic cache-list refactoring.

## What Changes

- Strengthen `main-decoder` extraction for migration-critical audio, UI, and sparse-subset families so the emitted decoder bundle materially covers the current fallback hot spots instead of only writing placeholder-empty assets.
- Expand decoder-authored sound-rule extraction for observed `kcs/sound/*` bucket families, especially `kc9997`, `kc9998`, and `kc9999`, so covered bucket members stop depending primarily on Rust-owned fallback tables.
- Update `emukc_bootstrap` `Rules`-path generation so covered decoder bundle members are generated from decoder assets first and legacy fallback is reserved for explicit residual members only.
- Keep verification centered on `decoder_cachelist_compare` so fallback shrinkage, residual families, and the small remaining ship/slot precision tail stay measurable after each regeneration.

## Non-goals

- Do not switch the default CLI/bootstrap strategy to `CacheListMakeStrategy::Rules`.
- Do not remove Rust fallback generators wholesale; this change only shrinks them where decoder bundle coverage becomes strong enough.
- Do not remove the `main-decoder/src/path-rules.ts` parity bridge in this change.
- Do not broaden this change into gameplay traits such as `SortieOps` or `MaterialOps`, or any KCSAPI route groups under `src/bin/net/router/kcsapi/`; no handler behavior changes are intended.
- Do not migrate broad `kcs2/img/*` versioned art families unless a targeted decoder bundle gap cannot be closed without them.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `decoder-coverage-assets`: tighten decoder output requirements for migration-critical audio, UI, gauge, and sparse-subset families so bundle artifacts are strong enough to replace the current highest-value Rust fallback families.
- `decoder-sound-rules`: extend decoder-authored `kcs/sound/*` rule coverage so observed `kc9997`, `kc9998`, and `kc9999` families stop depending primarily on Rust-owned bucket generators.
- `cache-manifest-integration`: change `Rules`-path generation so covered decoder bundle members are emitted from decoder assets first and legacy fallback only fills uncovered residual members.

## Impact

- `main-decoder/src/audio-resources.ts`, `ui-resources.ts`, `resource-id-sets.ts`, `cache-rules.ts`, `pipeline.ts`, and related tests under `main-decoder/test/`.
- `crates/emukc_bootstrap/src/make_list/source/kcs/mod.rs`, `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/*.rs`, and Rules-path attribution tests in `crates/emukc_bootstrap/src/make_list/`.
- `examples/decoder_cachelist_compare.rs` and regenerated comparison reports under `.data/` as verification artifacts.
- No changes to gameplay trait surfaces, `_impl` gameplay helpers, `entity::user` / `entity::profile` scope, or KCSAPI route handlers.
