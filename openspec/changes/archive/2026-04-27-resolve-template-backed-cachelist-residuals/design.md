## Context

Decoder-first cache-list generation is already wired end-to-end: `examples/decoder_cachelist_compare.rs` can build a candidate from `main-decoder/out/resources/cache_rules.json`, load sibling decoder assets, and report authority totals. The current report preserves full baseline recall (`baseline_only_count = 0`) but still marks migration as not ready because 6,156 candidate paths remain fallback-authored.

The largest blockers are template-backed families:

- `gauge`: 1,139 fallback-authored residuals
- `map`: 1,076 fallback-authored residuals
- `sound.kc9998`: 331 fallback-authored residuals
- `bgm`: 191 fallback-authored residuals

The current decoder asset model already exposes `resource_templates.json` with families such as `map.base`, `gauge.map`, `bgm.category`, and `sound.kc9998`. The Rust Rules path in `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs` already expands some template-backed output as rule-authored, but partial descriptors, missing runtime input validation, and broad fallback overlap still leave large residuals.

No gameplay traits, database entities, or KCSAPI route handlers are part of this change.

## Goals / Non-Goals

**Goals:**

- Make template-backed resource families explicit enough for the Rules path to decide whether a family is decoder-authoritative, partially covered, or fallback-owned.
- Resolve the high-impact `gauge`, `map`, `sound.kc9998`, and `bgm` residuals where decoder evidence and runtime inputs can safely prove ownership.
- Keep fallback residuals narrow and attributable when decoder evidence is incomplete.
- Preserve path-set recall and serialized cache-list item shape.
- Improve report diagnostics so the next blocker is actionable without manually inspecting thousands of paths.

**Non-Goals:**

- No default strategy switch to decoder-first.
- No deletion of legacy fallback generators in `emukc_bootstrap`.
- No Rust fallback constants copied into decoder outputs to claim completeness.
- No gameplay trait, SeaORM entity, or KCSAPI handler changes.
- No requirement that every small `useitem` or `area` residual be resolved in this change.

## Decisions

### Decision: Treat decoder templates as descriptors with explicit input validation

The decoder will continue to emit path templates, but template-backed ownership will depend on both descriptor completeness and runtime input availability. For example, `map.base` can be complete with `manifest.mapinfo`, while `sound.kc9998` may remain partial until a validated sound-bucket input exists.

Alternative considered: mark all non-unresolved templates as rule-authored and let fallback duplicate paths be ignored later. This hides real coverage gaps and risks claiming ownership for families whose member set is not proven.

### Decision: Expand complete families before fallback, then suppress duplicate fallback ownership

The Rules path will expand covered template families under `CacheListAuthorityStage::RuleAuthored` before invoking legacy fallback. Fallback may still run for continuity, but overlapping paths from complete decoder-owned template families must not be reported as fallback-authored residuals.

Alternative considered: skip fallback entirely for broad prefixes like `map` or `gauge`. That is too risky until the descriptor proves the full family boundary and required runtime inputs.

### Decision: Keep partial families visibly partial

Partial template families remain eligible to emit proven rule-authored paths, but residual paths outside the proven set remain fallback-authored and grouped under a family label plus reason. This is especially important for `sound.kc9998`, where decoded path shape exists but member enumeration may depend on a cache-source sound bucket.

Alternative considered: split this into separate map/gauge/sound changes. That would reduce blast radius, but the diagnostic and ownership mechanics are shared enough that doing the framework once is simpler and more testable.

### Decision: Use existing Codex and bootstrap inputs

Runtime input validation should use data already available to bootstrap generation, such as `ApiManifest` mapinfo, BGM, mapbgm, and existing cache-source data. If a needed input is not currently loaded, the descriptor must identify that gap rather than inventing membership.

Alternative considered: add new repo-tracked generated lists for every residual family. That would reduce runtime work but weakens the decoder-first authority model.

## Risks / Trade-offs

- Incorrectly marking a partial family complete -> Add comparison assertions for full baseline recall, fallback residual reduction, and representative path ownership.
- Duplicate path ownership remains inflated by fallback -> Add targeted Rust tests around overlapping template-expanded and fallback-generated paths.
- Sound bucket input is not reliable enough for completion -> Keep `sound.kc9998` partial and report the exact missing input rather than forcing readiness.
- Candidate-only count grows materially -> Compare before/after reports and inspect high-prefix deltas before marking tasks complete.
- Template descriptor schema churn affects existing assets -> Keep new fields backward-compatible or default missing fields to partial/fallback territory.

## Migration Plan

1. Record the current report metrics and template residual groups as the baseline.
2. Strengthen decoder template descriptors for the target families.
3. Update Rust template expansion and ownership accounting with focused tests.
4. Regenerate decoder assets and bootstrap synced assets.
5. Re-run `decoder_cachelist_compare` and confirm residual blockers shrink without `baseline_only_count` regression.
6. If a family cannot be proven complete, leave fallback enabled and record the blocker reason in diagnostics.

Rollback is straightforward: keep legacy fallback behavior unchanged and revert only the new decoder descriptor and ownership accounting changes.

## Open Questions

- Can `sound.kc9998` membership be validated from existing cache-source data, or does it need a separate sound-bucket input artifact?
- Are `gauge` image sidecars fully derivable from current manifest/map data, or do event variants require an additional map catalog input?
- Should readiness consider all fallback-authored paths, or only fallback-authored paths for families the decoder claims as covered?
