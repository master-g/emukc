## Context

The cache list generation pipeline (`make-list`) produces resource paths that the `populate` command attempts to download from CDN. In the latest run, 196 out of ~70K paths failed with "failed on all CDN" errors. Root cause analysis traced these failures to five independent bugs in path generation logic, all within `crates/emukc_bootstrap/src/make_list/`.

Current pipeline flow for the Rules strategy:
1. `make_cache_rules()` iterates `resource_manifest.entries` → `generate_entry_paths()` produces ship/slot/explicit paths
2. `add_decoder_template_paths()` expands template families (gauge, area, bgm, map, voice) from decoder assets
3. `make_manifest_support()` runs legacy fallback generators (gauge, unversioned, ship category extensions)

## Goals / Non-Goals

**Goals:**
- Eliminate all 196 cache-list failures caused by the five identified bugs
- Maintain backward compatibility with the serialized cache-list format
- Keep existing test coverage passing

**Non-Goals:**
- Fix `slot/item_character/0042_3621.png` failure (deferred — may be a genuine CDN gap)
- Refactor the overall template expansion architecture
- Add new decoder coverage families

## Decisions

### D1: Fix sortno=0 classification with `is_some_and(|s| s > 0)` in generate.rs

In `graph_group_ship_ids_from_cache_rules()` (generate.rs:213-218), change the friend_graph filter from `api_sortno.is_some()` to `api_sortno.is_some_and(|s| s > 0)`. The 15 shipgraph entries with `api_sortno=Some(0)` are graph-only placeholders not present in `api_mst_ship` — they have no character_full/character_up resources. This single filter change prevents ~60 invalid paths from being generated.

**Alternative considered**: Add a separate `is_friendly_ship()` helper. Rejected — the filter is used in exactly one place, and the intent is clear from the comparison.

### D2: Extend EVENT_SHIP_HOLES to include 6244-6262

Ships 6244-6262 exist in `api_mst_shipgraph` but not in `api_mst_ship`. They are unreleased placeholder entries with no CDN resources. Add them to `EVENT_SHIP_HOLES` in `ship.rs` and to `cache_rules.json`'s `eventShipHoles`. This prevents ~76 invalid paths.

**Alternative considered**: Dynamically exclude ships not in `api_mst_ship`. Rejected — the holes mechanism already exists and is the standard way to exclude event ships without resources.

### D3: Robust directory detection in explicit path generation

In `generate_explicit_paths()` (generate.rs:751), after the `ends_with('/')` check, add a heuristic: if a path has no file extension (no `.` in the last path segment) and doesn't end with `/`, skip it as a likely directory reference. This catches `"resources/voice"` and `"resources/friendly_panel/e"`.

**Alternative considered**: Fix the manifest JSON data to add trailing `/`. Not sufficient alone — future decoder runs could reintroduce the same pattern. Better to handle it in code.

### D4: Scope template area paths to observed decoder IDs

In `add_template_area_paths()` (mod.rs:261), instead of iterating ALL `api_mst_mapinfo` areas, use the decoder UI resource's observed area IDs when available. Fall back to the hardcoded `AREA_AIR_UNIT` list from `unversioned.rs` when decoder data is absent. This matches the behavior already used in `add_decoder_ui_paths()`.

### D5: Scope template gauge paths to known gauge map IDs

In `add_template_gauge_paths()` (mod.rs:231), replace the unconditional `api_mst_mapinfo` iteration with the union of `MAP_ID_LIST` and `EVENT_MAP_ID_LIST` from `gauge.rs`. These hardcoded lists represent the set of maps that actually have gauge files on CDN.

## Risks / Trade-offs

- **[D3 heuristic false positives]** → A path like `resources/some_resource` without extension but is a real file would be skipped. Mitigation: the only known cases are `voice` and `friendly_panel/e`, and real resource files in this codebase always have extensions.
- **[D5 hardcoded lists]** → New event maps with gauge files won't be auto-discovered. Mitigation: `EVENT_MAP_ID_LIST` is already maintained manually and updated with each event; the gauge module already requires manual updates for new events.
- **[D4 decoder dependency]** → If decoder UI assets are absent, the hardcoded fallback list (`AREA_AIR_UNIT = ["006","007","058"]`) is used, which is more conservative and correct.
