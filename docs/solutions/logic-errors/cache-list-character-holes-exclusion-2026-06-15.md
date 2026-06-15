---
title: "cache make-list emits slot 42 item_character that 404s on populate (character_hole_ids wired only behind #[cfg(test)])"
date: 2026-06-15
category: logic-errors
module: emukc_bootstrap
problem_type: logic_error
component: tooling
symptoms:
  - "cache populate reports kcs2/resources/slot/item_character/0042_3621.png (failed on all CDN) -> HTTP 404"
  - Sibling slotitems 0041/0043 return HTTP 200 for item_character, isolating the failure to id 42
  - "Only the item_character target 404s for slot 42; other targets (card, statustop_item, btxt_flat) succeed"
root_cause: logic_error
resolution_type: code_fix
severity: low
tags: [bootstrap, cache-list, make-list, character-holes, slotitem, item-character, cfg-test, path-rules]
related_components: [emukc_cache]
---

# cache make-list emits slot 42 item_character that 404s on populate

## Problem

`cache populate` tried to download `kcs2/resources/slot/item_character/0042_<suffix>.png` for slotitem 42 (応急修理要員) — an old item whose `item_character` art was never published to the game CDN — and failed with HTTP 404. The hole-exclusion data existed and was correct, but the production cache-list generator never consumed it, so the offline cache build emitted a path that can never be satisfied.

## Symptoms

- `cache populate` logged `kcs2/resources/slot/item_character/0042_3621.png (failed on all CDN)` → HTTP 404.
- Sibling slotitems return 200: `0041_*.png` and `0043_*.png` for `item_character` resolve fine, isolating the failure to id 42.
- Target-specific: slot 42 succeeds for other targets (`card`, `statustop_item`, `btxt_flat`); only `item_character` 404s, because that art asset genuinely doesn't exist.
- The 404 was one of ~16 populate failures during routine maintenance, but the only one caused by a code defect rather than not-yet-published CDN content.

## What Didn't Work

- **Treating all 16 populate 404s as one class.** They split cleanly into two buckets: 15 were genuinely-not-yet-published CDN content (new ships 743–745 `album_status`; new abyssal equipment 1658/1659), and exactly 1 (slot 42) was a code bug. Conflating them would have masked the real defect.
- **"Add 42 to the holes data."** Dead end — the data was already correct. `crates/emukc_bootstrap/assets/resource_manifest.json` contains `pathRules.characterHoleIds: [42]`, which deserializes into `PathRules.character_hole_ids` (documented in `types.rs` as *"Slotitem IDs that should be excluded from `item_character`"*). The data layer was right; the defect was a missing *consumer* in production.
- **Following the test-only path.** The only code that applied the hole filter — `slot.rs`'s `CHARACTER_HOLES` / `make_character_with_rules` — was gated behind `#[cfg(test)]`:

  ```rust
  #[cfg(test)]
  static CHARACTER_HOLES: LazyLock<Vec<i64>> = LazyLock::new(|| vec![42]);

  #[cfg(test)]
  fn make_character_with_rules(mst: &ApiManifest, list: &mut CacheList, rules: Option<&PathRules>) {
      let rules = rules.filter(|rules| !rules.character_hole_ids.is_empty());
      for m in mst.api_mst_slotitem.iter() {
          let is_hole = rules
              .map(|rules| rules.character_hole_ids.contains(&m.api_id))
              .unwrap_or_else(|| CHARACTER_HOLES.contains(&m.api_id));
          if m.api_sortno == 0 || is_hole { continue; }
          // ...emit slot/item_character/<id>_<suffix>.png
      }
  }
  ```

  So tests asserted the *intended* behavior while production `generate_slotitem_paths` never saw this code.
- **Assuming the holes system was entirely unwired.** It wasn't. The ship-side holes system *is* correctly wired into production via `should_skip_ship_category(...)` (consulting `event_ship_holes` / `enemy_ship_holes`). Only the slotitem `item_character` branch was missing its consumer.

## Solution

The fix lives in production `generate_slotitem_paths` (`crates/emukc_bootstrap/src/make_list/manifest/generate.rs`). After resolving slot IDs, when the target is `item_character` and `path_rules.character_hole_ids` is non-empty, retain only non-hole IDs (and early-return if that empties the set).

**Before** — `target` was inlined into the resolve call and `character_hole_ids` was never consulted:

```rust
let sources = entry.slot_mst_id_sources.as_deref().unwrap_or(&[]);
let slot_ids = resolve_slot_ids_for_target(
    sources,
    entry.target_type.as_str(),      // target only used here, then dropped
    mst, path_rules, decoder_assets, cache_rules,
);
if slot_ids.is_empty() {
    return;
}
let target = entry.target_type.as_str();
for id in slot_ids {
    // ... emits kcs2/resources/slot/item_character/0042_<suffix>.png
    //     for slot 42 — a file that does not exist on the CDN
}
```

**After** — `target` hoisted above the resolve call, hole filter applied before emission:

```rust
let sources = entry.slot_mst_id_sources.as_deref().unwrap_or(&[]);
let target = entry.target_type.as_str();
let mut slot_ids =
    resolve_slot_ids_for_target(sources, target, mst, path_rules, decoder_assets, cache_rules);
if slot_ids.is_empty() {
    return;
}

// Exclude item_character holes (e.g. slotitem 42 whose art is not on the CDN).
if target == "item_character"
    && let Some(holes) = path_rules.map(|rules| rules.character_hole_ids.as_slice())
    && !holes.is_empty()
{
    slot_ids.retain(|id| !holes.contains(id));
    if slot_ids.is_empty() {
        return;
    }
}

for id in slot_ids {
    // ...
}
```

**Regression test** (drives the *production* `generate_entry_paths` entry point, not the `#[cfg(test)]` helper):

```rust
#[test]
fn test_generate_slotitem_item_character_excludes_character_holes() {
    // slotitem 42's item_character art is not on the CDN; it is listed in
    // path_rules.character_hole_ids and must be excluded from generation.
    let mst = ApiManifest {
        api_mst_slotitem: vec![
            ApiMstSlotitem { api_id: 1, api_sortno: 1, api_version: Some(1), ..Default::default() },
            ApiMstSlotitem { api_id: 42, api_sortno: 42, api_version: Some(1), ..Default::default() },
        ],
        ..Default::default()
    };
    let path_rules = PathRules { character_hole_ids: vec![42], ..Default::default() };
    let entry = ResourceManifestEntry { /* kind: Slotitem, target_type: "item_character", ... */ };

    let mut list = CacheList::new();
    generate_entry_paths(&entry, &mst, Some(&path_rules), None, None, &mut list);

    let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
    assert!(paths.iter().any(|p| p.contains("slot/item_character/0001_")),
        "non-hole slotitem should be present");
    assert!(!paths.iter().any(|p| p.contains("slot/item_character/0042_")),
        "hole slotitem 42 should be excluded");
}
```

## Why This Works

The hole-exclusion feature spans three layers — data → load → generate — and the break was at the third link:

1. **Data layer (correct):** `resource_manifest.json` declares `pathRules.characterHoleIds: [42]`.
2. **Load layer (correct):** `populate_path_rules_locks` deserializes that into `PathRules.character_hole_ids: Vec<i64>` (the field on `PathRules` in `types.rs`).
3. **Generate layer (was the break):** production `generate_slotitem_paths` resolved slot IDs via `resolve_slot_ids_for_target(...)` and emitted a path for every resolved ID, never passing `character_hole_ids` into a filter. The only code that *did* filter — `slot.rs`'s `make_character_with_rules` — sat behind `#[cfg(test)]`, so it validated the intent in unit tests while leaving the real `cache populate` path emitting the doomed `0042_*.png`.

The fix closes the generate-layer gap by mirroring, in production, exactly the filter the test-only helper already applied. Because the data and load layers were already correct, no asset or schema change was needed — only the production consumer. This is also why the ship side worked and the slot side didn't: `should_skip_ship_category` had always been a production call site, whereas the slotitem `item_character` branch had only ever been exercised under `#[cfg(test)]`.

Verified end-to-end: regenerated cache list went 69876 → 69875 (exactly the slot 42 entry removed); `slot/item_character/0041_` and `0043_` unchanged; `cargo clippy -p emukc_bootstrap -- -W warnings` clean; `cargo fmt` clean; all crate tests pass.

## Prevention

- **No `#[cfg(test)]` filter without a production twin.** Any `#[cfg(test)]` helper that *mirrors* a production filtering decision is a hazard: it makes the test suite assert the intended behavior while leaving production free to diverge. Treat a test-only filter as a smell — require either the production generator to consume the same field, or delete the helper in favor of calling production code under test.
- **One entry point for holes.** Ship holes are honored through a single production predicate (`should_skip_ship_category`). Adopt the same shape for slot holes (e.g. a `should_skip_slot_category(target, id, path_rules)` predicate called from `generate_slotitem_paths`) so the ship and slot branches share an obvious structural twin, making a unilateral unwiring harder to reintroduce.
- **Production-integrated regression tests for every exclusion field.** Keep at least one test asserting production generation honors each `PathRules` exclusion (`character_hole_ids`, `event_ship_holes`, `enemy_ship_holes`, `btxt_flat_slot_ids`), driving the real `generate_entry_paths` entry point.
- **When `cache populate` reports 404s, classify before fixing.** A cluster of 404s usually mixes "code emitted a path it shouldn't have" (bug) with "CDN hasn't published the asset yet" (expected). Verify each failure against the CDN directly before treating them as one defect.

## Related Issues

- Strongest contextual anchor: `docs/superpowers/specs/2026-04-20-manifest-driven-makelist-design.md` (manifest-driven make_list design; names `CHARACTER_HOLES`, `item_character` as a `slotStandardCategory`, and `generate_entry_paths`). **Refresh candidate** — verify its hole-handling description now matches the production consumer.
- `openspec/specs/cache-manifest-integration/spec.md` — consider adding an explicit requirement that `item_character` generation MUST exclude `character_hole_ids` (currently only implied via sparse-subset semantics).
- No related GitHub issues found (searched `cache list holes`, `character_hole_ids`).
