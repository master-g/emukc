# Manifest-Driven Cache MakeList

## Context

The cache make-list system has two parallel paths: hardcoded templates in Rust (`slot.rs`, `generate.rs`) and decoder-extracted rules in JSON (`resource_manifest.json`, `battle_resource_rules.json`). The hardcoded path contains 165 btxt_flat IDs, 9 damage variant mappings, and multiple category lists that must be manually maintained when the game client updates. This design unifies both paths into a single manifest-driven approach where main-decoder is the sole authority for resource knowledge.

## Design

### 1. main-decoder Extensions

#### 1.1 Extend `resource_manifest.json` with `pathRules`

Add a `pathRules` block at the root level (version bump to 2):

```jsonc
{
  "version": 2,
  "pathRules": {
    "shipDamageVariants": {
      "banner": ["banner_dmg", "banner_g_dmg", "banner_g"],
      "banner2": ["banner2_dmg", "banner2_g_dmg", "banner2_g"],
      "banner3": ["banner3_g_dmg", "banner3_g"],
      "card": ["card_dmg"],
      "full": ["full_dmg"],
      "character_full": ["character_full_dmg"],
      "character_up": ["character_up_dmg"],
      "remodel": ["remodel_dmg"],
      "supply_character": ["supply_character_dmg"]
    },
    "shipStandardCategories": [
      "album_status", "banner", "banner_dmg", "banner_g", "banner_g_dmg",
      "banner2", "banner2_dmg", "banner2_g", "banner2_g_dmg",
      "banner3", "banner3_g", "banner3_g_dmg",
      "card", "card_dmg", "card_round",
      "character_full", "character_full_dmg",
      "character_up", "character_up_dmg",
      "icon_box", "power_up",
      "remodel", "remodel_dmg",
      "reward_card", "reward_icon", "special",
      "supply_character", "supply_character_dmg"
    ],
    "shipFullCategories": ["full", "full_dmg"],
    "slotStandardCategories": [
      "card", "card_t", "item_on", "item_on2",
      "item_up", "item_up2", "remodel", "statustop_item",
      "airunit_banner", "airunit_fairy", "airunit_name",
      "btxt_flat", "item_character"
    ],
    "enemyPlaneIds": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
  },
  "entries": [...]
}
```

The decoder extracts these by analyzing the game client code:
- **shipDamageVariants**: from `resources.getShip()` call patterns that branch on damage state
- **shipStandardCategories**: union of all `targetType` values for ship resources using the standard `{category}/{id}_{suffix}.png` pattern
- **shipFullCategories**: ship categories where path includes `api_filename`
- **slotStandardCategories**: union of all `targetType` values for slotitem resources
- **enemyPlaneIds**: from enemy plane resource references in the game code

#### 1.2 Extend `battle_resource_rules.json` per-rule metadata

Each rule gains:
```jsonc
{
  "id": "...",
  "resourceKind": "ship",
  "targetType": "full",
  "pathTemplate": "kcs2/resources/ship/{category}/{id:04}_{suffix}_{filename}.png",
  "usesApiFilename": true,
  "damageVariants": null
}
```

This is secondary to `pathRules` — the Rust side primarily uses `pathRules` from the manifest for category/validation lookups.

#### 1.3 btxt_flat coverage via manifest entries

The decoder ensures that all slot IDs requiring btxt_flat coverage appear as manifest entries with `kind: "slotitem"` and `targetType: "btxt_flat"`. The current 165 hardcoded IDs should be verifiable against decoder output.

### 2. Rust Changes

#### 2.1 Strategy enum simplification

**File**: `crates/emukc_bootstrap/src/make_list/mod.rs`

```rust
pub enum CacheListMakeStrategy {
    /// Skip resource list generation
    Minimal,
    /// Generate from resource_manifest.json
    Manifest(ManifestConfig),
}

pub struct ManifestConfig {
    /// Verify resources exist on remote CDN
    pub verify_remote: bool,
    /// Max concurrent remote checks
    pub concurrent: usize,
}
```

#### 2.2 New `PathRules` type

**File**: `crates/emukc_bootstrap/src/make_list/manifest/types.rs`

```rust
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct PathRules {
    #[serde(default)]
    pub ship_damage_variants: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub ship_standard_categories: Vec<String>,
    #[serde(default)]
    pub ship_full_categories: Vec<String>,
    #[serde(default)]
    pub slot_standard_categories: Vec<String>,
    #[serde(default)]
    pub enemy_plane_ids: Option<Vec<i64>>,
}
```

`ResourceManifest` gains a `path_rules: Option<PathRules>` field (optional for backward compat with v1 manifests).

#### 2.3 `generate.rs` — replace constants with `PathRules` lookups

**File**: `crates/emukc_bootstrap/src/make_list/manifest/generate.rs`

- Delete `SHIP_DAMAGE_VARIANTS`, `SHIP_STANDARD_CATEGORIES`, `SHIP_FULL_CATEGORIES`, `SLOT_STANDARD_CATEGORIES`
- `generate_entry_paths()` takes `&PathRules` parameter
- `get_damage_variants()` queries `path_rules.ship_damage_variants`
- Category checks use `path_rules.ship_standard_categories.contains()` etc.
- Enemy plane generation uses `path_rules.enemy_plane_ids`

#### 2.4 `slot.rs` — replaced by manifest entries

**File**: `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/slot.rs`

- Delete `make_default()`, `make_btxt_flat()`, `make_enemy_plane()`, `make_character()`
- Delete `BTXT_FLAT_IDS`, `CHARACTER_HOLES`, `ENEMY_PLANE_MAX_ID`
- Keep `make()` as a thin dispatcher: if strategy is Manifest, delegate to manifest generate; if Minimal, return empty
- `has_btxt_flat_coverage()` queries manifest entries instead of `BTXT_FLAT_IDS`

#### 2.5 CLI changes

**File**: `src/bin/cli/cache/make_list.rs`

- `--greedy` flag → `--verify-remote` (maps to `ManifestConfig.verify_remote`)
- `--manifest` becomes the default mode (no flag needed)
- `--minimal` remains for skip-all behavior
- `--concurrent` stays as-is

#### 2.6 Removal list

Files/functions to delete:
- `slot.rs::make_default()`
- `slot.rs::make_btxt_flat()`
- `slot.rs::make_btxt_flat_greedy()`
- `slot.rs::make_enemy_plane()`
- `slot.rs::make_enemy_plane_greedy()`
- `slot.rs::make_character()`
- `slot.rs::make_character_greedy()`
- `slot.rs::BTXT_FLAT_IDS`
- `slot.rs::CHARACTER_HOLES`
- `slot.rs::ENEMY_PLANE_MAX_ID`
- `generate.rs::SHIP_DAMAGE_VARIANTS`
- `generate.rs::SHIP_STANDARD_CATEGORIES`
- `generate.rs::SHIP_FULL_CATEGORIES`
- `generate.rs::_SP_REMODEL_CATEGORIES`
- `generate.rs::SLOT_STANDARD_CATEGORIES`

### 3. Verification

#### Phase 1: main-decoder
- `cd main-decoder && bun test` passes
- `bun run decode -- --sync-battle-assets` generates extended JSON
- `bun run decode -- --sync-resource-manifest` generates v2 manifest with `pathRules`
- Diff new JSON fields against current hardcoded values in Rust

#### Phase 2: Rust types
- `cargo test -p emukc_bootstrap` — new deserialization tests for `PathRules` and v2 manifest
- Existing tests in `generate.rs` updated to construct `PathRules` fixtures

#### Phase 3: Replace hardcoded logic
- `cargo test -p emukc_bootstrap make_list` — manifest-driven output matches old Default output
- `cargo test` full workspace passes
- `cargo clippy --workspace` clean

#### Phase 4: End-to-end
- `cargo run -- cache make-list` generates valid list
- `cargo run -- cache make-list --verify-remote` validates against CDN
- Compare output size and content with previous Default/Greedy runs

### 4. Migration Path

1. main-decoder: extract `pathRules`, generate v2 manifest
2. Rust: add `PathRules` type, backward-compat loading (v1 manifests work without `pathRules`)
3. Rust: wire `PathRules` into `generate.rs`, keep old constants as fallback for v1 manifests
4. Rust: `slot.rs` delegation to manifest, delete hardcoded functions
5. Rust: unify CLI flags
6. Remove v1 fallback after one release cycle
