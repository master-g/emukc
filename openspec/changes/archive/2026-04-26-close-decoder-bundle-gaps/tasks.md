## 1. Strengthen decoder bundle extraction

- [x] 1.1 Extend `main-decoder/src/audio-resources.ts` so live decoded modules produce non-empty categorized BGM coverage and stronger titlecall / explicit voice coverage for the current fallback-heavy sound families.
- [x] 1.2 Extend `main-decoder/src/cache-rules.ts` so observed `kc9997`, `kc9998`, and `kc9999` bucket members are preserved in sound rules instead of remaining effectively empty partial buckets.
- [x] 1.3 Extend `main-decoder/src/ui-resources.ts` so live decoded modules emit concrete map, useitem, area, and world-select members instead of near-empty assets.
- [x] 1.4 Extend `main-decoder/src/ui-resources.ts` furniture extraction so explicit decoder-observed furniture families cover the current `normal/movable/scripts/thumbnail/outside/reward/picture` residual hot spots where possible.
- [x] 1.5 Extend `main-decoder/src/resource-id-sets.ts` so decoder-observable sparse ship/slot subsets stop remaining unresolved when the decoded script already contains literal evidence.
- [x] 1.6 Upgrade `main-decoder` tests to assert meaningful live-output counts or representative real-family coverage instead of only toy regex fixtures or file-exists checks.

## 2. Narrow Rules-path fallback to uncovered residuals

- [x] 2.1 Update `crates/emukc_bootstrap/src/make_list/source/kcs/mod.rs` so decoder-covered sound bucket members are emitted as rule-authored first and Rust bucket generators fill only the uncovered remainder.
- [x] 2.2 Update `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/bgm.rs` and `unversioned.rs` so decoder-provided audio members reclaim ownership for covered BGM, titlecall, tutorial voice, and explicit voice families.
- [x] 2.3 Update `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/map.rs`, `gauge.rs`, `furniture.rs`, and `use_item.rs` so concrete decoder bundle members are emitted as decoder-authored output and fallback remains only for uncovered residual members.
- [x] 2.4 Add or update Rules-path attribution tests in `crates/emukc_bootstrap/src/make_list/` proving newly covered families shrink fallback-authored output without losing baseline recall.

## 3. Verify migration progress with compare reports

- [x] 3.1 Regenerate decoder outputs with `cd main-decoder && bun run decode -- --sync-assets`.
- [x] 3.2 Run `cd main-decoder && bun test` and targeted Rust tests for `emukc_bootstrap` after the extraction and Rules-path changes.
- [x] 3.3 Regenerate `.data/decoder_rules_compare_current.json` (or a successor report) with `cargo run --example decoder_cachelist_compare -- --config emukc.config.toml --rules main-decoder/out/resources/cache_rules.json --report-json ...`.
- [x] 3.4 Confirm the updated comparison report keeps `baseline_only_count = 0` while materially reducing `fallback_authored_candidate_count`, especially for `kc9998`, BGM, furniture, map, gauge, useitem, and titlecall residuals.

## 4. Guard ship and slot precision

- [x] 4.1 Recheck the current `candidate_only` ship and slot families after bundle-gap work and ensure the change does not regress the existing `229`-path precision tail.
- [x] 4.2 If the targeted decoder extraction work exposes obvious fixes for the current `ship/full` or slot alias over-generation families, capture those fixes within this change only when they are directly caused by the same bundle-gap work.
