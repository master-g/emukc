## Why

The decoder-first cache-list pipeline now reaches `100%` baseline recall with only `229` candidate-only paths, so accuracy is no longer the main blocker. The dominant remaining blocker is authority: `38685` candidate paths are still fallback-authored, and `kcs/sound/*` alone accounts for roughly `33786` of them, which means the sound domain is now the highest-leverage target for moving the cache list from parity mode into true decoder-first generation.

This is the right next step because it changes the strategic picture, not just the polish. Migrating `kcs/sound` from Rust-authored fallback logic into decoder-authored rules would move most of the remaining residual mass out of legacy scaffolding and make the decoder-first path materially closer to being the primary cache-list authority.

## What Changes

- Introduce decoder-authored sound rule outputs for the algorithmic `kcs/sound/*` families that are still generated from Rust-owned formulas, tables, and cache-source buckets.
- Extend Rust `Rules` generation so `kcs/sound/kc9997`, `kc9998`, `kc9999`, and ship voice paths are generated from the decoder rule bundle before legacy fallback logic is consulted.
- Make the decoder-first comparison loop report sound-rule migration progress explicitly so the remaining sound fallback share is visible and measurable.
- Preserve the current explicit audio coverage asset behavior for `se`, `bgm`, `titlecall`, and tutorial voice resources, while clarifying which audio domains are explicit-path assets versus algorithmic rule-driven sound families.

## Non-goals

- Do not switch the default cache-list strategy to `Rules` in this change.
- Do not broaden the work into map, furniture, gauge, or ship/slot cleanup beyond what is required to support sound migration.
- Do not redesign gameplay traits such as `SortieOps` or `MaterialOps`, or any `api_get_member` / `api_req_*` KCSAPI route groups.
- Do not remove the existing Rust fallback sound generators yet; they remain fallback and validation paths during this phase.

## Capabilities

### New Capabilities
- `decoder-sound-rules`: define decoder-authored semantic and algorithmic rules for `kcs/sound/*` cache-list families, including ship voice formulas, special voice families, and non-ship sound buckets currently owned by Rust fallback logic.

### Modified Capabilities
- `cache-manifest-integration`: decoder-driven cache-list generation requirements change so the `Rules` path must generate covered `kcs/sound/*` families from decoder-authored sound rules before consulting legacy fallback generators.
- `decoder-coverage-assets`: audio coverage requirements change to distinguish explicit audio path groups from algorithmic sound-rule families and to emit the metadata needed to drive decoder-authored `kcs/sound/*` generation.
- `decoder-cachelist-comparison`: comparison requirements change so the report must surface sound-domain fallback residuals and sound-rule migration progress explicitly.

## Impact

- Affected decoder extraction and output schema in `main-decoder/src/audio-resources.ts`, `main-decoder/src/cache-rules.ts`, `main-decoder/src/types.ts`, and `main-decoder/src/pipeline.ts`.
- Affected Rust rule loading and generation in `crates/emukc_bootstrap/src/make_list/manifest/` plus `crates/emukc_bootstrap/src/make_list/source/kcs/`.
- Affected validation workflow in `examples/decoder_cachelist_compare.rs` and `.data/decoder_rules_compare*.json`.
- No expected changes to Codex gameplay semantics, SeaORM entities, or HTTP/KCSAPI handler behavior.
