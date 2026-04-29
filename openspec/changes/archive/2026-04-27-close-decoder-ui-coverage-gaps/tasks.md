## 1. Establish Current UI Coverage Baseline

- [x] 1.1 Record current `ui_resources.json` summary counts for map, furniture, useitem, area, and world-select groups.
- [x] 1.2 Record current `decoder_cachelist_compare` fallback-authored prefixes for targeted UI families.
- [x] 1.3 Identify representative decoded modules or source snippets that expose map, furniture, useitem, area, and world-select resource evidence.

Current baseline:

- `ui_resources.json` summary: map default files 0, map event files 0, furniture categories 8, useitem card IDs 16, useitem card_ IDs 16, area sally IDs 0, area airunit IDs 0, world-select files 48.
- `decoder_cachelist_compare`: `baseline_only_count = 0`, `candidate_only_count = 250`, `fallback_authored_candidate_count = 6156`, no missing bundle assets, no repo fallback bundle assets.
- Largest targeted fallback residuals: `gauge (1139)`, `map (1076)`, `useitem.card (59)`, `useitem.card_ (38)`, `area (4)`.
- Representative decoded evidence: `module-2156-map-thumbnail-image.js` for map construction, `module-15478-furniture-loader.js` and `module-55242-fanimation-data.js` for furniture categories/paths, `module-79787-useitem-loader.js` for card/card_ paths, `module-1136-area-text-image.js`, `module-96928-air-unit-panel-banner.js`, and `module-6597-task-extend-air-unit.js` for area paths, plus supplemental `world.js` world-select ranges.

## 2. Strengthen `main-decoder` UI Extraction

- [x] 2.1 Extend `main-decoder/src/ui-resources.ts` map extraction for decoder-observable literal, concatenated, table, or deterministic construction patterns.
- [x] 2.2 Extend useitem extraction so observable `useitem/card` and `useitem/card_` IDs populate separate asset groups.
- [x] 2.3 Extend area extraction for observable `area/sally`, `area/airunit`, and `area/airunit_extend_confirm` members.
- [x] 2.4 Extend world-select extraction for observable filenames and deterministic file groups.
- [x] 2.5 Improve furniture explicit-path extraction for decoder-observable `normal`, `movable`, `scripts`, `thumbnail`, `outside`, `reward`, or `picture` members without copying Rust constants.

## 3. Add Decoder-Side Verification

- [x] 3.1 Add or update `main-decoder` tests for representative map and furniture extraction patterns.
- [x] 3.2 Add or update `main-decoder` tests for useitem, area, and world-select extraction patterns.
- [x] 3.3 Add a live-output or fixture-backed assertion that regenerated UI coverage no longer leaves all targeted groups empty when decoded evidence exists.

## 4. Preserve Rules-Path Ownership Semantics

- [x] 4.1 Review `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs` to ensure decoder UI members are added under rule-authored authority before fallback.
- [x] 4.2 Add or update Rust attribution tests proving decoder-covered UI paths remain rule-authored when legacy fallback emits overlapping paths.
- [x] 4.3 Keep fallback enabled for incomplete UI families and ensure uncovered residual paths remain fallback-authored.

## 5. Regenerate Assets and Verify Migration Progress

- [x] 5.1 Run `cd main-decoder && bun run decode -- --sync-assets` to regenerate decoder and bootstrap UI assets.
- [x] 5.2 Run `cd main-decoder && bun test`.
- [x] 5.3 Run targeted Rust tests for `emukc_bootstrap` cache-list attribution.
- [x] 5.4 Regenerate the decoder cache-list comparison report with `decoder_cachelist_compare`.
- [x] 5.5 Confirm `baseline_only_count = 0`, no missing sibling bundle assets, targeted UI fallback prefixes shrink, and `candidate_only_count` does not materially regress.

Final verification:

- `cd main-decoder && bun run decode -- --sync-assets`: regenerated decoder outputs and synced bootstrap assets for script version `6.2.8.0`.
- `cd main-decoder && bun test`: 61 pass, 0 fail.
- `cargo test -p emukc_bootstrap make_list::source::kcs2::resources`: 12 pass, 0 fail.
- `cargo run --example decoder_cachelist_compare -- --config emukc.config.toml --rules main-decoder/out/resources/cache_rules.json --report-json .data/decoder_rules_compare_current.json`: `baseline_only_count = 0`, `candidate_only_count = 250`, `fallback_authored_candidate_count = 6156`, no missing bundle assets, no repo fallback bundle assets.
- Remaining migration blockers are explicit residual ownership work: `gauge (1139)`, `map (1076)`, `sound.kc9998 (331)`, `bgm (191)`, `useitem.card (59)`, `useitem.card_ (38)`, and `area (4)`.
