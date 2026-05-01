## Why

The `populate` command fails to download 196 files (out of ~70K) because the cache list generation pipeline produces invalid resource paths. Five independent bugs cause these failures: directory paths treated as files, template over-expansion for gauge/airunit areas, incorrect friend/enemy ship classification for `sortno=0` graph entries, missing event ship holes for IDs 6244-6262, and an unfriendly-path in the explicit-path manifest.

## What Changes

- Fix `generate_explicit_paths()` to detect directory-like paths that lack a trailing `/` (e.g., `"resources/voice"`, `"resources/friendly_panel/e"`).
- Fix `graph_group_ship_ids_from_cache_rules()` to exclude `sortno=0` graph entries from friend_graph targets. These 15 entries (IDs 3-482) are not real playable ships and have no character_full/character_up resources.
- Update `EVENT_SHIP_HOLES` in `ship.rs` and `cache_rules.json` to include ships 6244-6262, which exist in shipgraph but not in api_mst_ship (unreleased placeholders).
- Constrain `add_template_area_paths()` so airunit/airunit_extend_confirm paths are only generated for observed area IDs, not all mapinfo areas.
- Constrain `add_template_gauge_paths()` so gauge JSON paths are only generated for maps that actually have gauge files (EO and event maps), not all mapinfo entries.

## Capabilities

### New Capabilities

_None._

### Modified Capabilities

- `decoder-first-cachelist-pipeline`: Fix friend/enemy ship ID resolution in `graph_group_ship_ids_from_cache_rules()` and template expansion for gauge/area paths.

## Impact

- `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` — explicit path filtering, ship ID resolution
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/ship.rs` — EVENT_SHIP_HOLES constants
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs` — template area/gauge expansion
- `crates/emukc_bootstrap/assets/cache_rules.json` — eventShipHoles update
- `crates/emukc_bootstrap/assets/resource_manifest.json` — optional data fix for friendly_panel/voice entries
