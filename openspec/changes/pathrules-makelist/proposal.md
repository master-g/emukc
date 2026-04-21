## Why

The cache make-list system maintains hardcoded constants in Rust (`generate.rs`, `slot.rs`, `ship.rs`) for ship damage variants, resource categories, btxt_flat slot coverage (336 IDs), enemy plane IDs, character holes, event/enemy ship holes, and more. These must be manually updated when the game client changes. The main-decoder already extracts this knowledge from `main.js` — a v2 `resource_manifest.json` with a `pathRules` block can replace all these constants with decoder-derived data, making the Rust side data-driven instead of hand-maintained.

## What Changes

- main-decoder emits a v2 `resource_manifest.json` with a `pathRules` block containing all category lists, damage variant mappings, hole lists, and coverage IDs currently hardcoded in Rust
- Rust gains a `PathRules` type that deserializes from `pathRules` (optional, backward-compatible with v1 manifests)
- Default/Greedy code paths in `generate.rs`, `slot.rs`, `ship.rs` read from `pathRules` when available, falling back to existing constants for v1 manifests
- `has_btxt_flat_coverage()` migrates to a manifest-derived `HashSet<i64>` with constant fallback
- No changes to strategy dispatch, CLI flags, or the Manifest strategy path

## Capabilities

### New Capabilities
- `pathrules-loading`: Deserialize v2 manifest `pathRules` into typed Rust structs, populate `OnceLock` for downstream access, backward-compatible with v1 manifests
- `pathrules-makelist-integration`: Wire `pathRules` into existing Default/Greedy make-list code paths to replace hardcoded constants with manifest-derived values

### Modified Capabilities

## Impact

- **main-decoder**: TypeScript changes to emit `pathRules` block in v2 manifest
- **emukc_bootstrap**: New `PathRules` type, `OnceLock` infrastructure, modified `generate.rs`/`slot.rs`/`ship.rs` to read from `pathRules`
- **emukc_bootstrap/battle_rules.rs**: `has_btxt_flat_coverage()` data source changes (function signature unchanged)
- **assets**: `resource_manifest.json` format bumps to v2 with new `pathRules` block
- No API changes, no database changes, no gameplay trait changes
