## Why

The decoder-driven rules path is now close enough to the current bootstrap baseline that the main blocker is no longer raw coverage, but system shape. We already have `100%` baseline recall and only a small `candidate-only` tail, yet cache-list generation still treats decoder rules as an experimental side path layered on top of legacy bootstrap semantics instead of the primary `main.js -> decode -> rules -> cache list` pipeline.

This is the right time to formalize that pipeline. Without an explicit decoder-first contract, the remaining precision work will keep getting mixed with legacy fallback behavior, and it will stay unclear which paths are truly rule-authored versus still inherited from built-in Rust logic.

## What Changes

- Define a decoder-first cache-list pipeline that consumes the decoder output bundle, together with `start2`, `kcs_const`, and cache version inputs, and produces a cache list through the existing bootstrap infrastructure.
- Add explicit authority and fallback accounting so generation can distinguish paths produced directly from decoder-authored rules from paths still produced by legacy parity fallback.
- Tighten the remaining high-noise residual families using decoder-side rule metadata instead of leaving them hidden behind broad fallback expansion.
- Extend the comparison workflow so migration progress is measured not just by overlap, but also by remaining fallback share and unresolved prefix clusters.
- Prepare the bootstrap path for an eventual default switch to decoder-first generation, while keeping the legacy `Default` and `Manifest` strategies intact during this change.

## Non-goals

- Do not make the decoder rules path the default CLI or bootstrap strategy in this change.
- Do not delete the existing legacy generators under `crates/emukc_bootstrap/src/make_list/source/`.
- Do not redesign unrelated gameplay traits such as `SortieOps` or `MaterialOps`, or any `api_get_member` / `api_req_*` KCSAPI route groups.
- Do not broaden the work into CDN existence probing, Greedy strategy changes, or unrelated cache domains with stable parity unless they are required to support the decoder-first pipeline contract.

## Capabilities

### New Capabilities
- `decoder-first-cachelist-pipeline`: define a first-class decoder-to-cache-list pipeline that accepts decoder rule assets plus runtime inputs (`start2`, `kcs_const`, `version`), tracks which output is rule-authored versus fallback-authored, and exposes migration readiness for bootstrap adoption.

### Modified Capabilities
- `cache-manifest-integration`: decoder-driven cache-list generation requirements change so decoder bundle inputs become the primary semantic authority for covered domains, while legacy behavior is preserved only as explicit fallback.
- `decoder-cachelist-comparison`: comparison requirements change so reports must measure fallback usage and migration readiness in addition to baseline overlap and prefix-group deltas.

## Impact

- Affected decoder output and schema code in `main-decoder/src/`, especially rule/coverage asset generation and emitted metadata.
- Affected bootstrap cache-list generation in `crates/emukc_bootstrap/src/make_list/manifest/` and strategy selection under `crates/emukc_bootstrap/src/make_list/source/`.
- Affected validation tooling in `examples/decoder_cachelist_compare.rs` and generated `.data/decoder_rules_compare*.json` reports.
- No expected changes to gameplay behavior, SeaORM entities, Codex structure, or KCSAPI handler routing.
