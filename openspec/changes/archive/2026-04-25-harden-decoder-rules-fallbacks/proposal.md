## Why

Recent review found that the decoder-first `Rules` path can treat partial decoder evidence as complete, keep inserting duplicate fallback-authored sound paths after covered decoder sound rules, and abort an otherwise usable rules bundle when optional sibling decoder JSON is malformed. These issues make the cache-list migration appear more authoritative than it is, while also making `--rules` brittle for partial decoder output directories.

## What Changes

- Make ship target semantic completeness explicit at the family level so partial `banner_g` / `banner2_g` / `banner3_g` signals cannot suppress legacy variant fallback for the whole banner family.
- Change Rules-mode `kcs/sound/*` generation so decoder-covered sound families suppress the corresponding legacy generators, and unresolved families keep fallback output clearly attributable as fallback.
- Make optional sibling decoder asset loading tolerant of missing, unreadable, and malformed JSON by warning and continuing with the remaining bundle data.
- Add focused regression coverage for partial decoder semantics, sound fallback suppression, and malformed optional sibling assets.
- Restore `cargo fmt --check` cleanliness for the changed Rust files.

## Non-goals

- Do not switch the default bootstrap strategy to `CacheListMakeStrategy::Rules`.
- Do not remove broad Rust fallback generators; this change only gates them when decoder coverage is explicit and complete enough.
- Do not expand decoder extraction to new audio/UI families beyond what is needed to fix the reviewed correctness gaps.
- Do not change gameplay traits such as `SortieOps` or `MaterialOps`, `_impl` gameplay helpers, SeaORM entities, or KCSAPI route groups.
- Do not address unrelated existing `bun run check` type-narrowing failures in `resource-id-sets.ts` or `pipeline.test.ts`.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `decoder-rule-semantics`: require target-family completeness to be proven before decoder ship semantics are treated as authoritative for legacy variant suppression.
- `decoder-sound-rules`: require covered sound-rule families to declare whether legacy fallback should be suppressed or preserved for unresolved residual coverage.
- `cache-manifest-integration`: require Rules-mode generation to tolerate malformed optional sibling decoder assets and to avoid duplicate fallback-authored sound paths for covered families.

## Impact

- `main-decoder/src/cache-rules.ts` and related tests for ship semantic completeness around banner-family targets.
- `crates/emukc_bootstrap/src/make_list/source/kcs/mod.rs`, `sound_rules.rs`, and make-list tests covering sound-rule fallback attribution.
- `crates/emukc_bootstrap/src/make_list/manifest/loader.rs` and loader tests covering optional sibling decoder assets.
- Rust formatting for changed files such as `crates/emukc_bootstrap/src/make_list/source/kcs/sound_rules.rs`, `crates/emukc_bootstrap/src/make_list/mod.rs`, and `examples/decoder_cachelist_compare.rs`.
