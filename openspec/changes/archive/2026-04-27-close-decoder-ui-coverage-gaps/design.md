## Context

The decoder-first cache-list path is structurally in place, but the current UI coverage asset is still mostly empty for high-value families. The latest generated `main-decoder/out/resources/ui_resources.json` reports zero map files, zero useitem IDs, zero area IDs, and zero world-select files, while `.data/decoder_rules_compare_current.json` still reports `fallback_authored_candidate_count = 8335`.

The largest UI-heavy fallback residuals include furniture `normal`, `movable`, `scripts`, and `thumbnail`; event/default map families such as `kcs2/resources/map/057` and `060`; gauge image families derived from map configs; useitem card resources; area resources; and world-select files. The relevant code paths are:

- `main-decoder/src/ui-resources.ts` for extracting UI coverage from decoded modules.
- `main-decoder/src/pipeline.ts` for writing and syncing `ui_resources.json`.
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs` for adding decoder UI paths as rule-authored output.
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/map.rs`, `gauge.rs`, `furniture.rs`, `use_item.rs`, and `unversioned.rs` for legacy fallback generation.
- `crates/emukc_bootstrap/src/make_list/mod.rs` for cache-list authority attribution.

No gameplay trait, database entity, Codex schema, or KCSAPI route behavior changes are needed. This is an internal decoder/bootstrap generation change.

## Goals / Non-Goals

**Goals:**

- Make `main-decoder` emit concrete decoder-observed UI coverage for map, furniture, useitem, area, and world-select families where decoded modules provide stable evidence.
- Preserve coverage modes so runtime-driven or incomplete groups remain partial or unresolved.
- Ensure Rules-path generation records decoder-provided UI members as rule-authored before running fallback.
- Reduce fallback-authored candidate paths for targeted UI prefixes while keeping `baseline_only_count = 0`.

**Non-Goals:**

- Do not change `CacheListMakeStrategy` defaults.
- Do not include BGM, `kcs/sound/*`, ship/slot precision, or path-rules removal in this change.
- Do not modify `SortieOps`, `MaterialOps`, `ShipOps`, `_impl` gameplay helpers, `entity::user`, `entity::profile`, or KCSAPI handlers.
- Do not synthesize UI coverage by copying Rust fallback constants into decoder assets.

## Decisions

### D1: Treat non-empty UI assets as the first milestone

`main-decoder/src/ui-resources.ts` should be improved against real decoded module patterns until the live asset emits representative concrete members for each targeted UI group. This comes before broad Rust-side fallback suppression.

Rationale: `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs` already adds decoder UI members under `RuleAuthored`; the current blocker is that the asset mostly has no members to add.

Alternative considered: start by changing fallback attribution in Rust. Rejected because it would move the metric without increasing decoder authority.

### D2: Keep extraction evidence-based and grouped by resource domain

The UI asset should continue storing domain-specific groups: map default/event files, furniture explicit paths/categories, useitem card/card_ IDs, area IDs, and world-select files. Extractors may recognize additional concatenation, template, array, table, or deterministic literal patterns from decoded modules, but must preserve the original resource grouping.

Rationale: bootstrap generation needs group-specific path templates, and compare reports are easier to interpret when each domain remains visible.

Alternative considered: flatten all UI paths into a single `explicitPaths` list. Rejected because it hides completeness semantics and makes residual fallback harder to narrow safely.

### D3: Narrow fallback only after concrete members are proven

Rules-path generation should continue running legacy fallback for incomplete families, but concrete decoder members must be inserted first and retain rule-authored attribution even if fallback later tries to add the same path.

Rationale: current `CacheList` authority tracking already preserves stronger rule-authored attribution for duplicate paths. The safe path is to reclaim proven members first, then narrow whole-family fallback only when the asset can prove completeness.

Alternative considered: skip fallback whenever a UI group is non-empty. Rejected because a non-empty partial group does not imply full family coverage.

### D4: Use compare reports as the acceptance gate

Implementation should be accepted only if `decoder_cachelist_compare` shows `baseline_only_count = 0`, no missing sibling bundle assets, and a targeted reduction in fallback-authored UI prefixes. `candidate_only_count` should not regress materially.

Rationale: this work is valuable only if it measurably moves ownership from Rust fallback to decoder-authored output while preserving recall.

Alternative considered: accept based on unit tests alone. Rejected because unit tests can prove patterns but not end-to-end cache-list ownership.

## Risks / Trade-offs

- [Risk] Decoded `main.js` may not expose enough stable evidence for full map, furniture, or gauge coverage. -> Mitigation: emit partial concrete members and keep fallback for the residual family.
- [Risk] Over-broad regex extraction can increase `candidate_only_count`. -> Mitigation: add representative extractor tests and gate with `decoder_cachelist_compare`.
- [Risk] Gauge coverage may depend on reading generated map/gauge JSON from cache rather than direct decoded UI paths. -> Mitigation: only reclaim gauge-adjacent members when the decoder output provides concrete, testable identifiers or paths.
- [Risk] This focused change overlaps conceptually with the broader active `close-decoder-bundle-gaps` change. -> Mitigation: keep this change limited to UI coverage and archive or merge in an order that avoids duplicate spec edits.

## Migration Plan

1. Strengthen `main-decoder` UI extraction and regenerate decoder outputs with `bun run decode -- --sync-assets`.
2. Add or update `main-decoder` tests that assert representative non-empty UI coverage from decoded fixtures or live-output checks.
3. Update Rules-path UI consumption only where concrete decoder members can be attributed as rule-authored without suppressing incomplete fallback.
4. Regenerate `decoder_cachelist_compare` and inspect targeted UI fallback prefixes.

Rollback is internal: revert the extractor, synced asset, and Rules-path attribution changes. No user-facing route, database, or gameplay behavior is changed.

## Open Questions

- Should gauge image ownership be modeled as part of the UI asset directly, or remain a map-derived fallback residual until decoded evidence is stronger?
- Which decoded module patterns should be treated as complete evidence for world-select resources versus partial explicit-file coverage?
