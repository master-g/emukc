## 1. PathRules Type & Loading Infrastructure

- [x] 1.1 Add `PathRules` struct to `crates/emukc_bootstrap/src/make_list/manifest/types.rs` with all fields: `ship_damage_variants`, `ship_standard_categories`, `ship_full_categories`, `slot_standard_categories`, `enemy_plane_ids`, `btxt_flat_slot_ids`, `character_hole_ids`, `event_ship_holes`, `enemy_ship_holes`, `special_ships`, `sp_remodel_ships`, `sp_remodel_mes`, `card_rounds`, `reward_ships`
- [x] 1.2 Add `#[serde(default)] pub path_rules: Option<PathRules>` to `ResourceManifest` struct
- [x] 1.3 Add `static PATH_RULES: OnceLock<PathRules>` and `static BTXT_FLAT_COVERAGE: OnceLock<HashSet<i64>>` to `crates/emukc_bootstrap/src/make_list/manifest/mod.rs`
- [x] 1.4 Add `pub(crate) fn path_rules() -> Option<&'static PathRules>` helper
- [x] 1.5 Populate OnceLocks after manifest load — when `path_rules` is `Some`, set `PATH_RULES` and build `BTXT_FLAT_COVERAGE` from `btxt_flat_slot_ids`
- [x] 1.6 Add deserialization tests: v1 manifest (no pathRules → `None`), v2 manifest with pathRules (all fields populated), v2 with partial fields (empty defaults)

## 2. Wire PathRules into Default/Greedy Code Paths

- [x] 2.1 In `crates/emukc_bootstrap/src/make_list/manifest/generate.rs`: add `path_rules: Option<&PathRules>` parameter to `generate_entry_paths()`, use `path_rules` values for damage variants and category lookups when present, fall back to constants when `None`
- [x] 2.2 In `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/slot.rs`: check `path_rules()` at function entry; use `enemy_plane_ids`, `btxt_flat_slot_ids`, `character_hole_ids` when available
- [x] 2.3 In `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/ship.rs`: check `path_rules()` at function entry; use `event_ship_holes`, `enemy_ship_holes`, `special_ships`, `sp_remodel_ships`, `sp_remodel_mes`, `card_rounds`, `reward_ships` when available
- [x] 2.4 Populate `PATH_RULES` OnceLock in `crates/emukc_bootstrap/src/make_list/source/mod.rs` at the start of the Default/Greedy branch (before calling `kcs::make()` and `kcs2::make()`)

## 3. has_btxt_flat_coverage Migration

- [x] 3.1 Update `has_btxt_flat_coverage()` in `slot.rs`: check `BTXT_FLAT_COVERAGE` OnceLock first, fall back to `BTXT_FLAT_IDS` constant
- [x] 3.2 Verify `battle_rules.rs:1095` caller unchanged — same `(slot_id: i64) -> bool` signature

## 4. Validation Tests

- [x] 4.1 Add test: deserialize v2 manifest, verify `pathRules` fields match current Rust constants (report discrepancies)
- [x] 4.2 Add test: `has_btxt_flat_coverage()` returns identical results for all 336 BTXT_FLAT_IDS entries with and without manifest
- [x] 4.3 Add test: Default strategy output with pathRules matches Default strategy output without pathRules
- [x] 4.4 Run `cargo test -p emukc_bootstrap`, `cargo test`, `cargo clippy --workspace` — all clean

## 5. main-decoder Extensions (separate repo/step)

- [x] 5.1 Extend main-decoder TypeScript to emit `pathRules` block in v2 `resource_manifest.json`
- [x] 5.2 Verify: `cd main-decoder && bun test` passes
- [x] 5.3 Verify: `bun run decode -- --sync-resource-manifest` generates v2 manifest with `pathRules`
