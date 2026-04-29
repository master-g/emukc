## Why

The decoder-first cache-list pipeline now reaches full baseline path recall, but the comparison still reports `8335` fallback-authored candidate paths. The remaining blocker is not broad pipeline plumbing; it is template-backed families where decoded `main.js` proves path formulas for UI, audio, map, gauge-adjacent, and furniture resources, while the decoder bundle still exposes mostly concrete lists or empty placeholders.

This change makes those template-backed families first-class decoder outputs so `Rules` generation can expand them from decoder-observed templates plus runtime bootstrap inputs and correctly attribute the resulting paths as rule-authored.

## What Changes

- Add decoder bundle metadata for template-backed resource families, including path templates, required runtime inputs, coverage mode, family identity, and decoded-module provenance.
- Extend `Rules` cache-list generation to expand decoder-observed templates with existing runtime manifest/cache inputs before using legacy Rust fallback.
- Attribute template-expanded paths as rule-authored only when the decoder evidence and runtime inputs are sufficient for that family.
- Preserve explicit fallback-authored residuals for families whose decoder template evidence, input binding, or completeness metadata remains partial or unresolved.
- Update comparison reporting so template-backed residuals and migration readiness are visible at the family/domain level.

## Non-goals

- Do not switch the default CLI/bootstrap cache-list strategy to `CacheListMakeStrategy::Rules`.
- Do not copy Rust fallback constants, CDN-discovered lists, or generated cache-list output into decoder assets as decoder evidence.
- Do not remove legacy fallback generators wholesale; fallback remains available for unresolved or unproven template families.
- Do not change gameplay traits such as `SortieOps`, `MaterialOps`, or `ShipOps`, and do not change KCSAPI route groups under `src/bin/net/router/kcsapi/`.
- Do not broaden this into unrelated gameplay, database, route-handler, or cache payload format changes.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `decoder-coverage-assets`: represent decoder-observed template-backed resource families with provenance, coverage mode, and runtime input requirements instead of only concrete file groups.
- `cache-manifest-integration`: expand decoder-observed templates in `Rules` generation and mark sufficiently proven output as decoder rule-authored.
- `decoder-first-cachelist-pipeline`: treat complete template-backed families as decoder-authoritative while keeping partial/unresolved families in explicit fallback territory.
- `decoder-cachelist-comparison`: report template-backed residuals and migration readiness so the next fallback blockers are actionable without inspecting raw path lists.

## Impact

- `main-decoder/src/ui-resources.ts`, `audio-resources.ts`, `cache-rules.ts`, `types.ts`, `pipeline.ts`, and related decoder tests.
- Decoder output assets such as `cache_rules.json`, `ui_resources.json`, `audio_resources.json`, and any synced equivalents under `crates/emukc_bootstrap/assets/`.
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/{map,gauge,furniture,bgm,unversioned,use_item}.rs`, `source/mod.rs`, bundle loading, and rule/fallback attribution paths.
- `examples/decoder_cachelist_compare.rs` and generated comparison reports under `.data/`.
- No gameplay trait surface, SeaORM entity, Codex data model, KCSAPI handler, or serialized cache-list item format changes are intended.
