## 1. Damage Variant Mapping

- [x] 1.1 Add `SHIP_DAMAGE_VARIANTS` static mapping table to `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` mapping base target types to their `_dmg`/`_g_dmg`/`_g` variants
- [x] 1.2 Add `get_damage_variants(base_type: &str) -> &[&str]` lookup function

## 2. Ship Path Generator Enhancement

- [x] 2.1 Modify `generate_ship_paths()` to generate damage variant paths when `damagedSource` is not `"false"` — for each base category, also generate paths for all mapped variants
- [x] 2.2 Handle `damagedSource = "true"` case: when the source is explicitly true, generate only the damage variant (not the base) for `full`/`full_dmg` style categories
- [x] 2.3 Ensure categories without damage variants (e.g., `album_status`, `special`, `power_up`) are unaffected

## 3. Tests

- [x] 3.1 Unit test: `get_damage_variants("banner")` returns `["banner_dmg", "banner_g_dmg", "banner_g"]`
- [x] 3.2 Unit test: ship entry with `damagedSource = "false"` produces only base paths
- [x] 3.3 Unit test: ship entry with `damagedSource = "_0x..."` (variable) produces base + all damage variants
- [x] 3.4 Unit test: ship entry with `damagedSource = "true"` produces only damage variant paths

## 4. Verification

- [x] 4.1 Run `cargo test -p emukc_bootstrap` — all tests pass
- [x] 4.2 Run `cargo clippy -p emukc_bootstrap` — no new warnings
- [x] 4.3 Run `cargo run -- cache make-list --manifest --overwrite` and verify ship coverage increased significantly vs previous 46%
- [x] 4.4 Restore default make-list after verification
