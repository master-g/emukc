## Why

The decoder-first cache-list pipeline now preserves full baseline recall, but migration readiness is blocked by fallback-authored residuals in template-backed resource families. The largest blockers are `gauge`, `map`, `sound.kc9998`, and `bgm`, where the decoder already emits path-template evidence but the Rules path still relies on legacy fallback for substantial output.

## What Changes

- Expand decoder template-backed resource metadata where decoded `main.js` proves resource path shape and runtime input requirements for map, gauge, BGM, and selected sound bucket families.
- Teach the Rules cache-list path to expand covered template families from decoder descriptors plus validated runtime inputs before invoking legacy fallback.
- Narrow fallback ownership so duplicate or now-covered template-backed paths are not reported as fallback-authored residuals.
- Improve comparison diagnostics so remaining template blockers identify the missing descriptor, completeness state, or runtime input gap instead of only broad path prefixes.
- Keep legacy fallback available for families whose decoder descriptor remains partial, unresolved, or missing required runtime inputs.

## Non-goals

- Do not switch the project default cache-list strategy to decoder-first.
- Do not remove legacy Rust fallback generators.
- Do not synthesize member completeness by copying fallback constants into decoder outputs.
- Do not attempt to resolve all UI and sound residual families in this change; small `useitem` and `area` residuals can remain fallback-authored unless directly affected by the template work.
- Do not change serialized cache-list item shape.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `decoder-coverage-assets`: refine template-backed resource metadata requirements for map, gauge, BGM, and sound bucket families.
- `cache-manifest-integration`: require Rules-path template expansion to use decoder descriptors plus validated runtime inputs before fallback for covered families.
- `decoder-first-cachelist-pipeline`: tighten authority accounting so complete template-expanded output is rule-authored and residual fallback is attributable by descriptor/input gap.
- `decoder-cachelist-comparison`: improve migration blocker reporting for template-backed residuals and readiness decisions.

## Impact

- Affected decoder code: `main-decoder/src/resource-templates.ts`, related extraction pipeline code, and decoder tests for template families.
- Affected bootstrap code: `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs`, `gauge.rs`, `map.rs`, `bgm.rs`, and sound-related Rules-path helpers.
- Affected diagnostics: `examples/decoder_cachelist_compare.rs` and report JSON fields for template-backed rule/fallback families.
- Affected data artifacts: `main-decoder/out/resources/resource_templates.json`, `cache_rules.json` only if sound bucket rule metadata changes, and synced bootstrap assets under `crates/emukc_bootstrap/assets/`.
- No gameplay traits or KCSAPI route groups are changed.
