## Why

The Manifest strategy (`cache make-list --manifest`) covers only 46% of ship resources because it does not generate damage variant paths (`_dmg`, `_g_dmg`, `_g`). The `resource_manifest.json` contains a `damagedSource` field per ship entry (e.g., `"false"`, `"true"`, `"damaged"`, obfuscated), but the Rust resolver only maps `"false"` and `"true"` to boolean — it does not produce the corresponding `_dmg`/`_g_dmg`/`_g` path suffixes. Fixing this closes the 13,987-path ship gap identified in the coverage analysis.

## What Changes

- Extend the ship path generator to produce damage variant paths (`_dmg`, `_g_dmg`, `_g`) for applicable target types when `damagedSource` is not explicitly `"false"`
- Add a mapping from base target types to their damage variants (e.g., `banner` → `banner_dmg`, `banner_g_dmg`; `card` → `card_dmg`; `full` → `full_dmg`)
- Handle `damagedSource` values beyond `"false"`/`"true"` — specifically `"damaged"` and obfuscated expressions (`_0x...`) should generate both normal and damage variants
- Increase Manifest strategy ship coverage from ~46% to ~90%+

## Capabilities

### New Capabilities

- `manifest-damage-variants`: Ship damage variant path generation in the manifest-driven cache list pipeline. Maps base target types to their `_dmg`/`_g_dmg`/`_g` counterparts and generates all applicable paths based on `damagedSource` resolution.

### Modified Capabilities

- `cache-manifest-integration`: The ship path generation requirement changes — instead of only generating the base target type path, the generator SHALL also produce damage variant paths based on a variant mapping table.

## Impact

- `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` — Ship path generation logic
- `crates/emukc_bootstrap/src/make_list/manifest/resolve.rs` — `damagedSource` resolution logic
- No changes to Default or Greedy strategies
- No changes to CLI or external APIs

## Non-goals

- Fixing coverage gaps for `kcs/` legacy voice/sound (separate future work)
- Fixing coverage gaps for `kcs2/img/` UI sprites (separate future work)
- Adding bgm/map/furniture/gauge/plane resource types to manifest (future phases)
- Removing or replacing Default/Greedy strategies
