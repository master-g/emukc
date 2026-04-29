## Context

The current decoder path already has the right ingredients, but not yet the right contract.

- `main-decoder/src/pipeline.ts` emits a decoder resource bundle under `out/resources/`, including `resource_manifest.json`, `resource_categories.json`, `resource_id_sets.json`, `audio_resources.json`, `ui_resources.json`, and `cache_rules.json`.
- `crates/emukc_bootstrap/src/make_list/source/mod.rs` already supports `CacheListMakeStrategy::Manifest` and `CacheListMakeStrategy::Rules`, but those paths are still threaded as special-case overrides inside the existing bootstrap generator.
- `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` now applies meaningful decoder-authored ship/slot semantics, yet the remaining pipeline still mixes rule-owned behavior and legacy fallback behavior inside the same generation flow.
- `examples/decoder_cachelist_compare.rs` measures overlap and grouped deltas, but it does not yet tell us how much of the candidate list is genuinely decoder-authored versus still inherited from fallback semantics.

That leaves the system in an awkward middle state: the `Rules` path is strong enough to validate against the baseline, but weakly modeled as a product surface. The desired end state is not just “high overlap”, but a stable pipeline where `main.js` decode output is the semantic authority, while legacy bootstrap logic becomes an explicit, shrinking fallback layer.

No gameplay trait (`SortieOps`, `MaterialOps`, etc.), database entity scope (`entity::user` / `entity::profile`), or KCSAPI route group needs to change for this work.

## Goals / Non-Goals

**Goals:**

- Define a first-class decoder-first cache-list pipeline across `main-decoder/src/` and `crates/emukc_bootstrap/src/make_list/`.
- Make rule-owned generation and legacy fallback generation explicit and measurable instead of implicitly mixed.
- Keep cache-list output format stable while adding sideband authority diagnostics that show what remains unresolved.
- Turn `examples/decoder_cachelist_compare.rs` into the migration gate for the eventual default-switch decision.

**Non-Goals:**

- Do not switch `src/bin/cli/cache/make_list.rs` or other CLI entrypoints to default to `CacheListMakeStrategy::Rules` in this change.
- Do not remove legacy generators under `crates/emukc_bootstrap/src/make_list/source/kcs*/`.
- Do not introduce new Codex fields or new `start2` loading behavior beyond what is already available through `Codex` and the existing manifest inputs.
- Do not change the serialized cache list line format consumed by downstream tooling.

## Decisions

### D1: Model decoder-first generation as an explicit bundle-driven pipeline

The Rust cache-list layer will treat decoder-driven generation as a bundle-driven pipeline rather than a loose collection of overrides. Concretely, work in `crates/emukc_bootstrap/src/make_list/manifest/types.rs`, `crates/emukc_bootstrap/src/make_list/manifest/loader.rs`, `crates/emukc_bootstrap/src/make_list/source/mod.rs`, and `crates/emukc_bootstrap/src/make_list/mod.rs` will center around a coherent decoder bundle made of:

- `cache_rules.json` as the primary rule/semantic asset
- sibling decoder coverage assets when present
- runtime manifest inputs already available through `Codex`
- cache-version inputs already used by path generation

Rationale:

- The decoder output is already emitted as a coherent bundle in `main-decoder/src/pipeline.ts`.
- Treating bundle inputs as first-class avoids reintroducing repo-asset fallback silently.
- This makes it possible to validate rule authority as a pipeline property rather than a set of helper calls.

Alternative considered: keep passing independent optional manifest/coverage/rules overrides into `build_list()`. Rejected because it preserves the current “hybrid by accident” shape and makes authority tracking fragile.

### D2: Track rule authority and fallback as sideband diagnostics, not cache-list payload changes

Cache-list output will remain the existing `{_id, path, version}` shape. Authority accounting will be collected separately during generation and surfaced through comparison/reporting structures in `crates/emukc_bootstrap/src/make_list/mod.rs` and `examples/decoder_cachelist_compare.rs`.

The sideband diagnostics should answer questions such as:

- how many generated paths were produced directly from decoder rules
- how many required legacy fallback expansion
- which prefixes or rule families still depend on fallback
- whether the candidate is migration-ready under explicit gates

Rationale:

- The cache list is a downloader input; it does not need provenance fields to function.
- Keeping the payload stable avoids changing downstream file consumers for a migration-internal concern.
- Sideband accounting can evolve more quickly than the cache list schema.

Alternative considered: attach authority metadata directly to `CacheListItem`. Rejected because it would change the output contract for little operational benefit and would complicate compatibility with existing list consumers.

### D3: Split decoder-first generation into rule phase first, fallback fill second

`crates/emukc_bootstrap/src/make_list/source/mod.rs` and `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` will be organized so decoder-first generation happens in two explicit phases:

1. Rule-owned expansion from decoder bundle semantics.
2. Residual fallback fill for categories still marked unresolved or explicitly outside decoder authority.

Each fallback-produced bucket must be recorded with a reason or category key so residual work remains visible.

Rationale:

- Today the pipeline still mixes broad fallback behavior into rule generation, which obscures how close we really are to a decoder-only path.
- Two explicit phases make the migration measurable and reduce the risk of fallback silently re-expanding newly constrained categories.

Alternative considered: continue with the current mixed expansion flow and infer fallback usage afterward from path prefixes. Rejected because post-hoc prefix heuristics will drift from actual generator behavior.

### D4: Use comparison reporting as the formal migration gate

`examples/decoder_cachelist_compare.rs` and the report types in `crates/emukc_bootstrap/src/make_list/mod.rs` will become the migration gate for decoder-first readiness. In addition to the current overlap and grouped delta metrics, reports should surface:

- rule-authored path count
- fallback-authored path count
- fallback-only prefix groups
- unresolved rule keys or uncovered families
- a migration-readiness summary derived from explicit thresholds

Rationale:

- The project already uses `decoder_cachelist_compare` as the iteration loop.
- Migration readiness should be data-driven, not a manual judgment from raw overlap numbers.

Alternative considered: keep using overlap-only reporting and inspect fallback behavior manually in code. Rejected because overlap alone cannot tell whether remaining parity comes from real decoder authority or from legacy scaffolding.

## Risks / Trade-offs

- [Risk] Authority tracking can drift from real generation behavior if it is reconstructed after the fact. → Mitigation: collect rule/fallback provenance at the generation call sites rather than from post-processed path prefixes.
- [Risk] Residual fallback categories may look “small enough” and stall cleanup work. → Mitigation: make fallback-prefixed reporting part of the comparison contract and use it as the next-change planning input.
- [Risk] Decoder bundle modeling may duplicate some information already present in repo-tracked assets. → Mitigation: treat decoder-output paths as canonical when explicitly provided and reserve repo assets for fallback/default flows only.
- [Risk] Readiness thresholds may be chosen too aggressively and block progress. → Mitigation: keep threshold policy explicit in reports and allow the exact switch criteria to remain configurable or follow-up work.

## Migration Plan

1. Introduce decoder-first bundle/context types and loader plumbing in `crates/emukc_bootstrap/src/make_list/manifest/` and `crates/emukc_bootstrap/src/make_list/mod.rs`.
2. Update generation flow in `crates/emukc_bootstrap/src/make_list/source/mod.rs` and `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` so rule-owned expansion and fallback fill are explicitly separated and classified.
3. Extend decoder asset emission in `main-decoder/src/cache-rules.ts`, `main-decoder/src/types.ts`, and `main-decoder/src/pipeline.ts` only as needed to support stable rule-bundle authority and residual reporting.
4. Extend `examples/decoder_cachelist_compare.rs` and comparison report structs so they expose authority/fallback metrics alongside existing overlap metrics.
5. Keep default CLI/bootstrap behavior unchanged; use the new reports to decide a later default-switch change.

Rollback strategy:

- Stop collecting authority diagnostics and fall back to the current hybrid `Rules` implementation.
- Continue loading `cache_rules.json` as today, but treat the new reporting fields as optional or ignore them entirely.
- Since this change does not switch defaults, rollback does not change user-facing bootstrap behavior.

## Open Questions

- Should migration readiness be a boolean gate in the JSON report, or only a structured threshold summary for humans and CI to interpret?
- Should fallback accounting be stored per generated path, or aggregated immediately by prefix/rule family to keep memory overhead smaller?
- Do we want `cache_rules.json` to carry explicit compatibility metadata tying the bundle to decoder script version and expected runtime inputs, or is the existing `scriptVersion` field sufficient for now?
