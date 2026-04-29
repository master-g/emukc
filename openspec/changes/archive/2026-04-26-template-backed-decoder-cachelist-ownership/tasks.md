## 1. Add Decoder Template Metadata

- [x] 1.1 Add `main-decoder` types for template-backed resource families, including family key, domain, path-template segments, required runtime inputs, coverage mode, and provenance.
- [x] 1.2 Extend decoder asset emission so template-backed metadata is written into the chosen bundle asset location and synced with bootstrap assets when the existing sync workflow is enabled.
- [x] 1.3 Extract template descriptors for map, gauge-adjacent map, furniture, useitem card/card_, area, and world-select families where decoded modules already prove deterministic path formulas.
- [x] 1.4 Extract template descriptors for BGM, titlecall, and high-value `kcs/sound/*` bucket families where decoded modules prove path shape or bucket structure.
- [x] 1.5 Add `main-decoder` tests proving descriptors include stable family keys, declared runtime inputs, explicit completeness state, and decoded-module provenance.

## 2. Load And Expand Templates In Bootstrap

- [x] 2.1 Add Rust deserialization and validation for decoder template-backed family descriptors in the decoder bundle loading path.
- [x] 2.2 Implement a restricted typed template expander for approved placeholder/input bindings without evaluating raw JavaScript or arbitrary expressions.
- [x] 2.3 Wire template expansion into `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/map.rs` and `gauge.rs` for covered map and gauge-adjacent families.
- [x] 2.4 Wire template expansion into `furniture.rs`, `use_item.rs`, `bgm.rs`, and `unversioned.rs` for covered furniture, useitem, BGM, titlecall, area, and world-select families.
- [x] 2.5 Wire template or bucket expansion into the relevant `kcs/sound/*` generation path for covered `kc9998`-style sound residuals.

## 3. Preserve Ownership And Fallback Semantics

- [x] 3.1 Mark paths expanded from complete decoder template descriptors and validated runtime inputs as rule-authored output.
- [x] 3.2 Suppress broad legacy fallback for complete template-backed families so duplicate path strings do not inflate fallback-authored ownership.
- [x] 3.3 Preserve fallback-authored residual output for partial, unresolved, or input-missing template families with attributable family labels.
- [x] 3.4 Add Rust tests covering complete template expansion, missing input fallback, partial-family residual fallback, and duplicate fallback suppression.

## 4. Improve Comparison Reporting

- [x] 4.1 Extend `examples/decoder_cachelist_compare.rs` report data to include template-backed rule-authored counts by family or domain.
- [x] 4.2 Extend fallback residual reporting so unresolved template-backed families and missing input bindings are distinct from generic fallback prefixes.
- [x] 4.3 Update migration-readiness logic so fallback-authored template-backed residuals remain explicit blockers until resolved.
- [x] 4.4 Add or update comparison report tests or fixtures covering template-expanded output and unresolved template residuals.

## 5. Regenerate And Verify

- [x] 5.1 Regenerate decoder outputs with the existing `main-decoder` decode and sync workflow.
- [x] 5.2 Run `main-decoder` tests for the template metadata extractors.
- [x] 5.3 Run targeted `emukc_bootstrap` Rust tests for decoder bundle loading, template expansion, and fallback attribution.
- [x] 5.4 Regenerate the decoder rules comparison report and confirm `baseline_only_count` remains `0`.
- [x] 5.5 Confirm `fallback_authored_candidate_count` drops from the current `8335` baseline for the targeted template-backed families without materially regressing `candidate_only_count`.
