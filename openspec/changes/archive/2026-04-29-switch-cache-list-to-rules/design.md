## Context

EmuKC's cache list generation has four strategies: `Default`, `Minimal`, `Greedy`, `Manifest`, and `Rules`. The `Default` strategy uses hardcoded Rust logic to enumerate resource paths for ships, slots, sound, BGM, maps, furniture, and other domains. The `Rules` strategy (`CacheListMakeStrategy::Rules`) was added incrementally over 5+ commits to consume decoder-extracted `cache_rules.json` and related assets.

Current comparison results show `Rules` achieves **100% baseline coverage** (69,229/69,229 paths, 0 baseline-only) with 91.6% rule-authored authority (63,654/69,479 paths). The remaining 8.4% fallback-authored paths are correct residuals (gauge images, map event variants, static BGM IDs) that the decoder does not yet cover.

The legacy `Default` path in `source/mod.rs` calls `kcs::make(... strategy ...)` and `kcs2::make(...)` without decoder assets. The `Rules` path calls `kcs::make(... Rules ... rules_bundle)` and `kcs2::make_manifest_support(...)` with the decoder bundle.

## Goals / Non-Goals

**Goals:**
- Make `CacheListMakeStrategy::Default` behave identically to `Rules`
- Remove legacy hardcoded fallback branches from `source/mod.rs`, `source/kcs/`, and `source/kcs2/resources/`
- Retain `Greedy` as `Rules` + holes reporting
- Update comparison example baseline to `Manifest`
- Update tests

**Non-Goals:**
- Removing `Manifest` strategy
- Reducing fallback-authored paths (future decoder iteration)
- Changing `cache_rules.json` schema
- Modifying KCSAPI handlers or gameplay traits

## Decisions

### D1: `Default` delegates to `Rules` at the strategy dispatch level

**Decision**: In `source/mod.rs::make()`, merge the `Default`/`Greedy` branches into the `Rules` branch. When `Default` or `Greedy` is selected, load the rules bundle and execute the `Rules` code path.

**Alternative considered**: Keep `Default` as a separate enum variant that internally constructs a rules bundle from embedded constants. Rejected — this duplicates state that already lives in `cache_rules.json`.

**Rationale**: The comparison data proves equivalence. Delegating avoids code duplication and ensures future decoder improvements automatically flow to `Default`.

### D2: `Greedy` wraps `Rules` + holes reporting

**Decision**: `Greedy` becomes `Rules` execution followed by the existing holes report generation. No separate code path.

**Rationale**: Greedy's only difference from Default was generating a holes report file. The path generation itself was identical.

### D3: Legacy `kcs2::make()` function removed

**Decision**: Remove `source/kcs2/resources/mod.rs::make()` (the function handling `Default`/`Greedy` without decoder assets). All strategies go through `make_manifest_support()` with the loaded decoder bundle.

**Rationale**: `make()` contained the original hardcoded path generation for bgm, furniture, gauge, map, ship, slot, unversioned, and use_item. `make_manifest_support()` already handles all of these with decoder-driven logic plus fallback where needed.

### D4: Legacy `kcs::make()` branches simplified

**Decision**: In `source/kcs/mod.rs::make()`, remove the non-`Rules` branch (the `else` block that calls `kc9997::make`, `kc9998::make`, `kc9999::make`, `purchase::make`, `voice::make` without decoder rules). All strategies now pass through the `Rules` branch which uses `sound_rules::make()` with fallback guardrails.

**Rationale**: The `sound_rules::make()` function already handles partial coverage by falling back to legacy generators. The non-`Rules` branch is redundant.

### D5: Comparison example default baseline becomes `Manifest`

**Decision**: Change `decoder_cachelist_compare.rs` default `--baseline` from `default` to `manifest`. This preserves the ability to measure decoder coverage against a non-decoder baseline.

**Rationale**: After this change, `Default` and `Rules` produce identical output, making `--baseline default` a no-op comparison. `Manifest` remains the useful comparison point.

### D6: Retain `Minimal` strategy unchanged

**Decision**: `Minimal` stays as-is. It produces a subset of resources for quick boot scenarios.

**Rationale**: Minimal is orthogonal to this change — it intentionally produces fewer paths.

## Risks / Trade-offs

**[Risk] Rules bundle must be loadable for Default to work** → The `load_cache_rules_bundle()` call will fail if `cache_rules.json` is missing from the assets directory. Mitigation: the file is tracked in the repo at `crates/emukc_bootstrap/assets/cache_rules.json` (263K). If missing, the error is clear and actionable (`run: cd main-decoder && bun run decode`).

**[Risk] Tests that assert Default behavior may break** → Integration tests that use `CacheListMakeStrategy::Default` will now produce `Rules` output (which includes the 250 extra candidate-only paths). Mitigation: update test assertions to match the new output.

**[Risk] Resolving `slotMstIdSources` warnings in log** → The comparison run showed 12 warnings about unknown expressions like `this._after_item_id`, `this._plane_mst_id`, etc. These are already skipped silently. No action needed for this change, but noted for future decoder iterations.

**[Accepted] 8.4% fallback ceiling is fundamental** → Exploration confirmed the remaining fallback-authored paths cannot be eliminated through decoder rules:
- BGM: `bgm.playBattleBGM(id)` receives IDs dynamically from `api_mst_mapbgm` lookup + hardcoded 257-ID empirical list. The decoder's regex patterns (`bgm.play(\d+, "battle")`) match nothing because battle BGM IDs are never literal in main.js.
- Gauge: base JSON paths derive from `api_mst_mapinfo` (already template-covered), but `_2`/`_3` variant suffixes and PNG image names require downloading and parsing each gauge JSON file — a two-phase process incompatible with pure rule generation.
- These hardcoded enumerations represent accumulated server-side knowledge and are the correct mechanism for these domains.

## Migration Plan

1. Modify `source/mod.rs` — collapse `Default`/`Greedy` into `Rules` path
2. Remove `kcs2::make()` legacy function
3. Simplify `kcs::make()` strategy dispatch
4. Update `Greedy` to wrap `Rules` + holes
5. Update comparison example default baseline
6. Run comparison to confirm identical output post-refactor
7. Update tests
8. `cargo test` + `cargo clippy`

No data migration needed — `cache_rules.json` is already tracked in the repo.
