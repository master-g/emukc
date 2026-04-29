## Context

The decoder-first cache-list work already has a `Rules` strategy that consumes `cache_rules.json` and sibling decoder coverage assets. The reviewed implementation still has three correctness gaps:

- `main-decoder/src/cache-rules.ts` can mark `targetSemantics` observed-complete from any `banner_g` / `banner2_g` / `banner3_g` evidence, even when that evidence is only a partial signal for the broader banner family.
- `crates/emukc_bootstrap/src/make_list/source/kcs/mod.rs` continues to run all legacy sound fallback generators after decoder sound rules, so duplicate paths can be inserted as fallback-authored output for families already covered by decoder rules.
- `crates/emukc_bootstrap/src/make_list/manifest/loader.rs` treats malformed optional sibling JSON as a fatal rules-bundle error even though missing or unreadable optional sibling assets are supposed to be tolerated.

This change is implementation-level in `main-decoder` and `emukc_bootstrap`. It does not add methods to gameplay traits such as `SortieOps` or `MaterialOps`, does not change `_impl` gameplay functions, does not touch `entity::user` or `entity::profile`, and does not affect KCSAPI route groups.

## Goals / Non-Goals

**Goals:**

- Preserve fallback safety when decoder ship target evidence is partial.
- Prevent decoder-covered sound families from being reinserted by broad legacy sound fallback generators.
- Treat malformed optional sibling decoder JSON like missing or unreadable optional coverage data: warn, mark that domain as fallback territory, and continue.
- Add regression tests for the reviewed P1/P2 cases and restore `cargo fmt --check` cleanliness.

**Non-Goals:**

- Do not broaden decoder extraction to new resource families beyond these correctness fixes.
- Do not remove Rust fallback generators or change the default bootstrap strategy.
- Do not fix unrelated TypeScript type-narrowing issues already reported by `bun run check`.
- Do not change runtime gameplay, database, or API behavior.

## Decisions

### D1: Track ship semantic completeness per target family, not per observed target

`main-decoder/src/cache-rules.ts` should only emit authoritative ship target semantics for a family when the decoder has enough evidence to cover that family's intended variant behavior. A partial signal for `banner_g`, `banner2_g`, or `banner3_g` must remain partial or unresolved instead of causing the decoder to emit the full hardcoded `SHIP_TARGET_SEMANTIC_CASES` set as observed-complete.

Rationale:

- Rust treats observed-complete target semantics as authoritative and skips legacy variant expansion.
- Family-level completeness prevents one damaged-only observation from suppressing valid fallback paths for sibling banner targets.

Alternative considered: keep the existing observed-complete flag and special-case banner targets in Rust. Rejected because completeness is decoder evidence metadata; Rust should consume that metadata, not infer decoder confidence from target names.

### D2: Gate legacy sound fallback generators by decoder sound-rule coverage

`crates/emukc_bootstrap/src/make_list/source/kcs/mod.rs` should decide which legacy sound fallback generators are still needed after applying `sound_rules`. Covered complete families should skip their matching fallback generator. Partial or unresolved families should keep fallback, and any paths generated there must remain attributable as fallback output.

Rationale:

- `CacheListItem` identity includes `_id`, so duplicate path strings from different authorship sources can coexist and later be written from `list.items`.
- Covered families should not report fallback-authored counts solely because a broad legacy generator ran after decoder rules.

Alternative considered: deduplicate by path before writing output. Rejected because that would hide authorship errors in comparison reporting while leaving the fallback execution model wrong.

### D3: Load optional sibling decoder assets with domain-local error handling

`crates/emukc_bootstrap/src/make_list/manifest/loader.rs` should load optional sibling assets through a helper that returns `None` for absent, unreadable, or malformed files after logging enough context to diagnose the bad asset. Required assets, including the selected `cache_rules.json`, should remain fatal on malformed input.

Rationale:

- Optional sibling data is used to improve coverage, not to define the presence of the rules bundle itself.
- Partial decoder output directories are expected during migration and comparison work.

Alternative considered: only tolerate `NotFound` and permission/read errors, leaving JSON parse errors fatal. Rejected because a stale or partially written sibling asset should not prevent testing the rest of the decoder bundle.

### D4: Keep verification targeted and serial where needed

Regression coverage should include:

- TypeScript tests proving partial banner-family evidence does not produce complete authoritative ship semantics.
- Rust make-list tests proving covered sound families do not receive duplicate fallback-authored paths.
- Rust loader tests proving malformed optional sibling JSON logs or returns fallback territory without aborting the rules bundle.
- `cargo fmt --check` after formatting changed Rust files.

`cargo test -p emukc_bootstrap make_list -- --test-threads=1` remains the reliable targeted Rust test shape while the existing parallel `Db(DatabaseAlreadyOpen)` issue is outside this change.

## Risks / Trade-offs

- [Risk] A family marked partial may temporarily increase fallback-authored output. Mitigation: this is preferable to under-generation; compare reports should distinguish fallback safety from decoder authority.
- [Risk] Suppressing a legacy sound fallback for the wrong covered-family key could drop valid paths. Mitigation: gate suppression only on explicit complete decoder sound-rule coverage and add regression tests around covered versus unresolved families.
- [Risk] Malformed optional assets could hide decoder pipeline bugs. Mitigation: warning messages should include the asset domain and path, while required `cache_rules.json` remains fatal.
- [Risk] Formatting cleanup may touch files adjacent to unrelated local work. Mitigation: run formatting narrowly where possible and review the diff before completion.

## Migration Plan

1. Fix decoder ship semantic completeness in `main-decoder/src/cache-rules.ts` and add targeted tests.
2. Update Rules-mode sound generation in `crates/emukc_bootstrap/src/make_list/source/kcs/mod.rs` and `sound_rules.rs` tests so complete decoder coverage suppresses matching fallback.
3. Update optional sibling asset loading in `crates/emukc_bootstrap/src/make_list/manifest/loader.rs` and add malformed JSON coverage.
4. Format touched Rust files and run targeted TypeScript and Rust verification.

Rollback strategy:

- Revert the decoder completeness change if it incorrectly marks proven families partial.
- Re-enable the affected legacy sound fallback generator if regression tests or comparison reports show lost baseline paths.
- Revert optional loader tolerance only for the affected sibling asset domain if warning-and-continue behavior masks an actually required asset.

## Open Questions

- Should optional malformed JSON warnings be emitted through the existing logging path only, or should comparison reports also expose skipped optional domains?
- Does `sound_rules` already expose the right complete/partial family key for every legacy generator, or does the Rust representation need a small helper for coverage lookup?
