## Context

The current decoder-driven comparison loop is already useful, but the numbers show two separate bottlenecks:

- Ship and slot paths are partially understood, with `79.11%` baseline coverage in those domains, but sparse categories are still badly over-generated because `crates/emukc_bootstrap/src/make_list/manifest/resolve.rs` treats many expressions as universal ship or slot selectors.
- Overall Default coverage is only `34.67%` because the decoder-driven candidate does not yet contribute major non-ship/slot domains such as sound, map, furniture, BGM, voice, and useitem.

There is also a workflow constraint: fast iteration currently depends on `examples/decoder_cachelist_compare.rs`, but that example only accepts an explicit `resource_manifest.json` override. Once decoder-driven generation depends on additional assets, the comparison loop must consume the whole decoder output bundle from `main-decoder/out/resources/` without forcing repo-asset sync on every run.

The current relevant code paths are:

- `main-decoder/src/resource-manifest.ts`: broad ship/slot/texture-provider/explicit-path extraction
- `main-decoder/src/resource-categories.ts`: existing category-group extraction
- `main-decoder/src/pipeline.ts` and `main-decoder/src/cli.ts`: decoder artifact emission and sync flow
- `crates/emukc_bootstrap/src/make_list/manifest/resolve.rs`: manifest selector expansion
- `crates/emukc_bootstrap/src/make_list/manifest/generate.rs`: manifest entry path generation
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/*`: current Default/Greedy hardcoded generators for ship, slot, BGM, unversioned UI resources, etc.
- `examples/decoder_cachelist_compare.rs`: current comparison loop

## Goals / Non-Goals

**Goals:**

- Emit a decoder asset bundle that captures sparse ship/slot subsets plus audio/UI domains needed for cache-list generation.
- Keep `resource_manifest.json` as the broad ship/slot pattern source, while using sibling assets to constrain sparse categories and expand missing domains.
- Extend Rust cache-list generation so decoder-driven candidate generation can meaningfully improve overall coverage, not just ship/slot overlap.
- Preserve a fast, repo-safe iteration loop where `decoder_cachelist_compare` can compare against decoder output assets directly.

**Non-Goals:**

- Replacing Greedy probing or treating decoder output as an exhaustive oracle for every cache-list domain.
- Changing gameplay traits, database entities, or KCSAPI route groups.
- Removing existing fallback constants from `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/*.rs` in the same step that introduces new decoder assets.
- Reworking battle-rule extraction in `crates/emukc_bootstrap/src/battle_rules.rs`.

## Decisions

### D1: Keep `resource_manifest.json` focused; add sibling decoder coverage assets

The decoder will continue to use `main-decoder/src/resource-manifest.ts` for broad ship/slot/texture-provider discovery, and it will keep `main-decoder/src/resource-categories.ts` as the source of deterministic category groups. New decoder work will add sibling assets for sparse subset metadata and non-ship/slot domains instead of overloading the manifest schema itself.

This produces a decoder output bundle under `main-decoder/out/resources/`:

```text
resources/
├── resource_manifest.json
├── resource_categories.json
├── resource_id_sets.json
├── audio_resources.json
└── ui_resources.json
```

Rationale:

- `resource_manifest.json` already models broad call-pattern extraction well.
- Sparse subsets and audio/UI domains have different semantics than manifest entries and need their own completeness/provenance fields.
- Separate assets let Rust fall back per domain instead of treating one malformed file as a fatal failure.

Alternative considered: extend `resource_manifest.json` with sparse-subset and audio/UI sections. Rejected because it mixes different abstraction levels and makes manifest consumers harder to reason about.

### D2: Decoder-driven generation stays under `CacheListMakeStrategy::Manifest`

Rust will not introduce a new top-level strategy just for these assets. Instead, `CacheListMakeStrategy::Manifest` will become an asset-augmented decoder-driven generation path. It will load `resource_manifest.json` plus any sibling decoder coverage assets and generate the candidate list from the combined bundle.

Concretely, the integration stays centered in:

- `crates/emukc_bootstrap/src/make_list/source/mod.rs`
- `crates/emukc_bootstrap/src/make_list/manifest/generate.rs`
- new helper loaders under `crates/emukc_bootstrap/src/make_list/manifest/` or an adjacent module dedicated to decoder coverage assets

Rationale:

- The user-facing concept is still “build candidate list from decoder output”.
- Existing comparison/example code already targets `Manifest` as the decoder candidate strategy.
- Avoids proliferating strategies before the asset model stabilizes.

Alternative considered: add a new `DecoderCoverage` or `ManifestAugmented` strategy. Rejected for now because it adds API surface without changing the underlying role of the strategy.

### D3: Sparse ship/slot categories use explicit subset selectors, not universal manifest resolution

`crates/emukc_bootstrap/src/make_list/manifest/resolve.rs` currently expands many ship and slot expressions to all friendly ships or all slotitems. That is acceptable for categories such as `album_status` or `card`, but it is incorrect for sparse categories such as `special`, `sp_remodel/*`, `card_round`, and `reward_*`.

The new `resource_id_sets.json` will therefore model sparse categories explicitly, with:

- stable category keys
- resource domain (`ship` / `slot`)
- observed ID lists
- provenance
- coverage mode (`observed-complete`, `partial`, `unresolved`)

Rust generation will use those subset selectors before falling back to universal manifest resolution for sparse categories.

Rationale:

- This addresses the biggest current over-generation buckets directly.
- It keeps the decoder honest about what it actually observed.
- It allows sparse categories and universal categories to coexist in the same manifest-driven path generator.

Alternative considered: keep the current universal resolver and rely on Greedy or comparison reports to prune. Rejected because it preserves large false-positive buckets and slows iteration.

### D4: Deterministic category gaps come from `resource_categories.json`

Some categories already exist in decoder outputs but do not appear as concrete manifest entries, such as ship `power_up` and slot `card_t`. Rust will use `resource_categories.json` as a deterministic complement to `resource_manifest.json` for these category-group driven paths.

This means the augmented manifest path will use:

- `resource_manifest.json` for resolved ship/slot entry patterns
- `resource_categories.json` for deterministic generation groups such as `power_up` and `card_t`
- `resource_id_sets.json` for sparse subset constraints

Rationale:

- These category gaps are already observable in decoder outputs today.
- Reusing `resource_categories.json` avoids inventing duplicate asset concepts.
- It closes obvious ship/slot misses without weakening the manifest model.

Alternative considered: duplicate these groups into `resource_id_sets.json`. Rejected because they are category-group concerns, not sparse subset concerns.

### D5: Audio and UI domains are added as first-class decoder coverage domains

The decoder will add:

- `audio_resources.json` for SE, categorized BGM, and voice/titlecall/tutorial resource groups
- `ui_resources.json` for map, furniture, useitem, area, and world-select resources

Rust consumption will extend the manifest-driven path so it can add these domains via the existing non-ship/slot generators under `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/`.

Rationale:

- The comparison results show that most remaining overall coverage is outside ship/slot.
- Audio and UI domains are currently the highest-return domains for overall coverage gains.
- These assets can be consumed with domain-local fallbacks, preserving current behavior where decoder coverage is partial.

Alternative considered: postpone audio/UI until ship/slot is perfect. Rejected because overall coverage would remain capped around the low-40% range even with near-perfect ship/slot results.

### D6: The comparison example consumes sibling assets and reports domain metrics

`examples/decoder_cachelist_compare.rs` will keep its explicit manifest-path entrypoint, but it will derive sibling assets from the same decoder output root and feed the full bundle into candidate generation. The report will also include domain-level breakdowns, not just global path-set overlap.

Desired loop:

```text
main-decoder decode
    ↓
out/resources/{manifest + coverage assets}
    ↓
decoder_cachelist_compare
    ↓
global overlap + domain overlap + sparse-category deltas
    ↓
next decoder/bootstrap iteration
```

Rationale:

- Keeps the fast iteration workflow local to decoder outputs.
- Avoids forcing repo-asset sync for every experiment.
- Domain metrics turn broad overlap numbers into actionable next targets.

Alternative considered: keep the example manifest-only and sync other assets into bootstrap manually. Rejected because it slows the iteration loop and increases the chance of comparing stale data.

## Risks / Trade-offs

- **Decoder sees only source-observable data** → Mitigation: encode `partial` / `unresolved` states explicitly and preserve Rust fallbacks for those categories.
- **Multiple decoder assets may drift out of sync** → Mitigation: emit them from the same decoder run under the same output root and use that root as the comparison/example input contract.
- **Manifest strategy grows more complex** → Mitigation: keep asset loading modular inside `crates/emukc_bootstrap/src/make_list/manifest/` and make each domain optional.
- **Audio/UI extraction may still miss irregular paths** → Mitigation: preserve domain-level comparison reporting so extraction work can target the highest-impact misses first.
- **Sparse subset mistakes can cause false negatives instead of false positives** → Mitigation: only let subset assets constrain categories that are explicitly marked as observed-complete; unresolved subsets keep fallback behavior.

## Migration Plan

1. Extend `main-decoder/src/pipeline.ts`, `main-decoder/src/types.ts`, and new extractor modules so the decoder emits the full coverage asset bundle into `main-decoder/out/resources/`.
2. Update `examples/decoder_cachelist_compare.rs` to consume sibling assets from decoder output and emit domain-level metrics.
3. Add Rust-side loader/helpers under `crates/emukc_bootstrap/src/make_list/manifest/` to read decoder coverage assets from bootstrap assets or manifest-adjacent overrides.
4. Extend manifest-driven generation in `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` and related `source/kcs2/resources/*.rs` helpers to:
   - use category groups for deterministic gaps
   - use sparse subsets for sparse ship/slot categories
   - add audio/UI domains when corresponding assets are present
5. Keep all existing Default/Greedy fallbacks in place until comparison output shows the decoder-driven path is stable enough to replace them category by category.

Rollback strategy:

- Stop emitting or loading the new decoder coverage assets.
- Candidate generation reverts to the current manifest-only path plus existing Default/Greedy fallbacks.
- The comparison example still works with manifest-only candidate input.

## Open Questions

- Should `coverageMode` be a small enum (`observed-complete`, `partial`, `unresolved`) or should it also capture whether a subset is safe to constrain generation automatically?
- Should manifest-adjacent asset loading be keyed by the manifest file path alone, or should the example accept an explicit decoder output root override for nonstandard layouts?
- Which slot categories beyond `btxt_flat` and `item_up` need subset metadata immediately, and which should stay fallback-only in the first implementation pass?
