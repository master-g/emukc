## 1. Manifest Type Definitions (emukc_bootstrap)

- [x] 1.1 Create `crates/emukc_bootstrap/src/make_list/manifest/types.rs`: define `ResourceManifest`, `ResourceManifestEntry`, `ManifestEntryKind` enum (ship/slotitem/texture-provider/explicit-path), and supporting structs matching the JSON schema in `crates/emukc_bootstrap/assets/resource_manifest.json`
- [x] 1.2 Add `#[derive(Deserialize)]` with serde defaults for forward-compatibility; unknown `kind` values deserialize to a catch-all variant
- [x] 1.3 Create `crates/emukc_bootstrap/src/make_list/manifest/mod.rs` re-exporting types

## 2. Manifest Loader

- [x] 2.1 Implement `load_resource_manifest()` in `crates/emukc_bootstrap/src/make_list/manifest/loader.rs`: read `crates/emukc_bootstrap/assets/resource_manifest.json`, deserialize to `ResourceManifest`, validate `version` field, emit warning on version mismatch
- [x] 2.2 Return descriptive error when file is missing, suggesting `bun run decode -- --sync-resource-manifest`

## 3. Expression Resolver

- [x] 3.1 Create `crates/emukc_bootstrap/src/make_list/manifest/resolve.rs`: implement `resolve_ship_ids(source: &str, mst: &ApiManifest) -> Vec<i64>` mapping known patterns to "all friendly ships" (ships with `api_sortno` or `api_aftershipid`); unknown patterns emit `warn!` and return empty
- [x] 3.2 Implement `resolve_slotitem_ids(sources: &[String], mst: &ApiManifest) -> Vec<i64>` for slotitem entries, same pattern-matching approach
- [x] 3.3 Implement `resolve_damaged(source: &str) -> Option<bool>` mapping `"false"` → Some(false), `"true"` → Some(true), unknown → None (generate both variants)

## 4. Path Generator

- [x] 4.1 Create `crates/emukc_bootstrap/src/make_list/manifest/generate.rs`: implement `generate_ship_paths(entry, ship_ids, damaged, mst, list)` using `SuffixUtils` and the same path templates as `source/kcs2/resources/ship.rs` (album_status, banner, card, full, etc.)
- [x] 4.2 Implement `generate_slotitem_paths(entry, slotitem_ids, mst, list)` using path templates from `source/kcs2/resources/slot.rs`
- [x] 4.3 Implement `generate_explicit_paths(entry, list)` adding paths verbatim to the cache list

## 5. Manifest Strategy Integration

- [x] 5.1 Add `CacheListMakeStrategy::Manifest` variant to the enum in `crates/emukc_bootstrap/src/make_list/mod.rs`
- [x] 5.2 Wire Manifest strategy into `source::make()` in `crates/emukc_bootstrap/src/make_list/source/mod.rs`: load manifest, resolve entries, generate paths, add to list
- [x] 5.3 Add CLI support for the Manifest strategy in the `cache make-list` command handler

## 6. Tests

- [x] 6.1 Unit test: manifest deserialization from sample JSON (all 4 entry kinds)
- [x] 6.2 Unit test: expression resolver — known patterns return correct ID sets, unknown patterns return empty with warning
- [x] 6.3 Unit test: path generator output matches expected patterns (compare with existing hardcoded paths for overlap)
- [x] 6.4 Unit test: missing manifest file returns error with helpful message
- [x] 6.5 Integration test: Manifest strategy produces a cache list with entries for ship, slotitem, and explicit-path kinds

## 7. Verification

- [x] 7.1 Run `cargo test -p emukc_bootstrap` — all tests pass
- [x] 7.2 Run `cargo clippy --workspace` — no new warnings
- [x] 7.3 Run `cargo run -- cache make-list --strategy manifest` and verify output contains ship/slotitem/explicit paths without HTTP HEAD checks
