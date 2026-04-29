## 1. Establish Residual Baseline

- [x] 1.1 Record current `decoder_cachelist_compare` totals for baseline-only, candidate-only, rule-authored, fallback-authored, migration readiness, and template fallback families.
- [x] 1.2 Capture representative fallback-authored path samples for `gauge`, `map`, `sound.kc9998`, and `bgm`.
- [x] 1.3 Inspect current `resource_templates.json` descriptors for target families and note coverage modes, required inputs, and provenance.

Current baseline:

- Compare totals: `baseline_only_count = 0`, `candidate_only_count = 250`, `rule_authored_candidate_count = 63323`, `fallback_authored_candidate_count = 6156`, `migration_ready = false`.
- Template rule-authored families: `furniture (1197)`, `sound.kc9998 (412)`, `bgm (200)`, `voice.titlecall (167)`, `map (144)`, `worldselect (48)`, `gauge (36)`, `area (21)`, `useitem.card_ (3)`, `useitem.card (2)`.
- Template fallback families: `gauge (1139)`, `map (1076)`, `sound.kc9998 (331)`, `bgm (191)`, `useitem.card (59)`, `useitem.card_ (38)`, `area (4)`.
- Representative target path samples from the current candidate path listing:
  - `gauge`: `kcs2/resources/gauge/00101.json`, `kcs2/resources/gauge/00102.json`, `kcs2/resources/gauge/00301.json`.
  - `map`: `kcs2/resources/map/001/01.png`, `kcs2/resources/map/001/01_image.json`, `kcs2/resources/map/001/01_info.json`.
  - `sound.kc9998`: `kcs/sound/kc9998/27305852.mp3`, `kcs/sound/kc9998/27605571.mp3`, `kcs/sound/kc9998/28405991.mp3`.
  - `bgm`: `kcs2/resources/bgm/battle/001_6601.mp3`, `kcs2/resources/bgm/battle/002_8038.mp3`, `kcs2/resources/bgm/battle/003_5332.mp3`.
- Current target descriptors: `map.base` is `observed-complete` with `manifest.mapinfo`; `map.info` is `partial` with `manifest.mapinfo`; `gauge.map` is `partial` with `manifest.mapinfo`; `bgm.category` is `observed-complete` with `manifest.bgm` and `manifest.mapbgm`; `sound.kc9998` is `partial` with `cache-source.sound-bucket`.

## 2. Strengthen Decoder Template Descriptors

- [x] 2.1 Extend `main-decoder` template extraction so target families expose explicit completeness blockers when they remain partial.
- [x] 2.2 Improve `map.base` and related map template descriptors to distinguish complete base map paths from sidecar or variant paths.
- [x] 2.3 Improve `gauge.map` descriptors to represent observed gauge path shapes and required map/runtime inputs without copying Rust fallback constants.
- [x] 2.4 Review `bgm.category` descriptor output against decoded evidence and manifest/mapbgm input requirements.
- [x] 2.5 Review `sound.kc9998` descriptor output and decide whether existing cache-source sound bucket input can validate membership.
- [x] 2.6 Add or update `main-decoder` tests for target template descriptors, coverage mode, required inputs, provenance, and blocker metadata.

## 3. Expand Template Families in Rules Path

- [x] 3.1 Update `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs` template expansion to use descriptor completeness and validated runtime inputs before fallback.
- [x] 3.2 Update map template expansion to emit decoder-authoritative paths for proven `map` members while leaving unproven sidecars or variants fallback-authored.
- [x] 3.3 Update gauge template expansion to emit decoder-authoritative gauge paths only when descriptor and runtime inputs prove ownership.
- [x] 3.4 Update BGM template expansion to align rule-authored output with `manifest.bgm` and `manifest.mapbgm` inputs.
- [x] 3.5 Update sound bucket handling for `sound.kc9998` so proven members are rule-authored and missing membership remains an explicit fallback residual.
- [x] 3.6 Add Rust attribution tests proving overlapping template-expanded and legacy fallback paths keep rule-authored ownership.

## 4. Improve Template Diagnostics

- [x] 4.1 Extend decoder-first sideband diagnostics with stable template family labels, rule-authored counts, fallback-authored counts, and residual reasons.
- [x] 4.2 Update `examples/decoder_cachelist_compare.rs` JSON output to preserve residual reasons for template-backed fallback blockers.
- [x] 4.3 Update human-readable comparison output so blocker summaries identify missing descriptor evidence, partial coverage, unavailable runtime input, or uncovered residual membership.
- [x] 4.4 Add comparison/report tests or fixture assertions for template-backed ownership and readiness behavior.

## 5. Regenerate and Verify

- [x] 5.1 Run `cd main-decoder && bun run decode -- --sync-assets` to regenerate decoder outputs and synced bootstrap assets.
- [x] 5.2 Run `cd main-decoder && bun test`.
- [x] 5.3 Run targeted Rust tests for `emukc_bootstrap` resource template expansion and attribution.
- [x] 5.4 Regenerate `.data/decoder_rules_compare_current.json` with `decoder_cachelist_compare`.
- [x] 5.5 Confirm `baseline_only_count = 0`, candidate-only count does not materially regress, target template residuals shrink, and any remaining blockers include explicit residual reasons.
