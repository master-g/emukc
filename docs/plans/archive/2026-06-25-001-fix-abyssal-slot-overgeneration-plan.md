---
title: "fix: exclude abyssal equipment from standard slot resource categories"
status: completed
type: fix
date: 2026-06-25
origin: session diagnosis of cache populate 404s (deep-sea equipment 1658/1659) — 2026-06-25
---

# fix: exclude abyssal equipment from standard slot resource categories

## Summary

`cache make-list` over-generates slot resource paths for abyssal (深海) equipment.
The `enemy_slot_border` (1500) that already excludes abyssal items from the
`item_up` and `btxt_flat` categories is **not** applied to the remaining standard
slot categories (`card`, `card_t`, `item_on`, `item_character`, `statustop_item`,
`remodel`, …), which fall through to `resolve::resolve_slotitem_ids` and only
filter on `api_sortno > 0`. Abyssal equipment with `sortno > 0` therefore gets
player-side graphic paths it has no resources for, and populate fails them on
every CDN (404).

The fix applies the existing `enemy_slot_border` exclusion to the fall-through
path so all standard slot categories honor the same abyssal boundary already used
by `item_up`/`btxt_flat`. This is a pure Rust make-list change — the border value
is already present in `cache_rules.json`; no decoder regen is needed.

## Problem Frame

Observed populate failures (subset, all 404 "failed on all CDN"):

```
kcs2/resources/slot/remodel/1658_5365.png
kcs2/resources/slot/card/1658_7174.png
kcs2/resources/slot/item_on/1659_8851.png
kcs2/resources/slot/item_character/1659_7782.png
kcs2/resources/slot/card_t/1659_8675.png
kcs2/resources/slot/statustop_item/1658_6418.png
…
```

Root cause, traced in `crates/emukc_bootstrap/src/make_list/manifest/generate.rs`:

- `resolve_slot_ids_for_target` applies `enemy_slot_border` **only** in two
  branches: `item_up` (remaps `id > border`) and `btxt_flat` (filters
  `id <= border`).
- Every other standard slot category returns the final fall-through
  `resolve::resolve_slotitem_ids(sources, mst)`, which filters **only**
  `api_sortno > 0` (`crates/emukc_bootstrap/src/make_list/manifest/resolve.rs`).

Confirmed scope of the defect against `.data/codex/start2.json`:

- Slot items with `api_sortno > 0` **and** `api_id > 1500`: exactly **2** —
  `1658` (深海標準14inch/45 三連装砲) and `1659` (深海標準14inch/45 連装砲).
- These two × 6 standard categories = the 12 slot 404s. The failure list
  contains **no** `item_up`/`item_up2` entries — confirming those branches
  already exclude the items correctly.
- Highest friendly equipment id (`sortno > 0`, `id <= 1500`) is **575**, far
  below the 1500 border → `id <= border` is a clean, friendly-safe boundary.
- The other 155 abyssal items (`id > 1500`) have `sortno = 0` and are already
  dropped by the `sortno > 0` filter.

## Requirements

- R1. Standard slot resource categories must not generate paths for abyssal
  equipment (`api_id > enemy_slot_border`), matching the exclusion `item_up` and
  `btxt_flat` already apply.
- R2. Friendly equipment (`api_id <= enemy_slot_border`) coverage is unchanged —
  no friendly slot path is dropped.
- R3. When `enemy_slot_border` is absent from the rules, behavior is unchanged
  (no filtering), preserving current output for assets that don't define it.
- R4. The `item_up` / `btxt_flat` / `airunit_*` / observed-subset branches are
  unaffected.

## Key Technical Decisions

- **KTD1 — Filter the fall-through, reuse the existing border source.** Apply the
  exclusion at the single final fall-through return of `resolve_slot_ids_for_target`,
  reading the border the same way `btxt_flat` already does:
  `cache_rules.and_then(|r| r.slot_rules.item_up.enemy_slot_border).unwrap_or(i64::MAX)`,
  then keep only `api_id <= border`. `i64::MAX` fallback satisfies R3. Do **not**
  push the filter into `resolve::resolve_slotitem_ids` itself — that helper is also
  called inside the `item_up` branch (which does its own `id > border` remap), and
  filtering there would change `item_up` semantics.
- **KTD2 — Exclusion, not remap.** Standard categories use exclusion (`id <= border`),
  not the `id - border` remap `item_up` uses. Abyssal equipment has no player-side
  `card`/`item_on`/etc. resource, so the correct action is to drop it, not remap it.
- **KTD3 — No decoder/asset change.** `enemy_slot_border = 1500` already exists in
  `cache_rules.json` (`slotRules.itemUp.enemySlotBorder`). This is a consumer-side
  fix only; no `main-decoder` regen.

## Implementation Units

### U1. Apply enemy_slot_border to the standard slot fall-through

**Goal:** Exclude abyssal equipment (`api_id > enemy_slot_border`) from all standard
slot categories by filtering the final fall-through of `resolve_slot_ids_for_target`.

**Requirements:** R1, R2, R3, R4

**Dependencies:** none

**Files:**
- `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` — modify the final
  fall-through return of `resolve_slot_ids_for_target` (the
  `resolve::resolve_slotitem_ids(sources, mst)` tail); add a regression test in the
  existing `#[cfg(test)] mod tests`.

**Approach:** Implement KTD1. Read `enemy_slot_border` from
`cache_rules.slot_rules.item_up.enemy_slot_border` with `unwrap_or(i64::MAX)`, then
filter the resolved ids to `api_id <= border` before returning. Leave the `item_up`,
`btxt_flat`, `airunit_*`, and observed-subset branches untouched. Mirror the border
read already present in the `btxt_flat` branch so the two stay consistent.

**Patterns to follow:** the `btxt_flat` branch in the same function
(`slot.api_id <= enemy_slot_border` with the `unwrap_or(i64::MAX)` border read).

**Test scenarios:**
- Happy path / R1: a standard category (e.g. `card`) resolved against a manifest
  containing one friendly slot (`api_id <= border`, `sortno > 0`) and one abyssal
  slot (`api_id > border`, `sortno > 0`) → resolved ids include the friendly id and
  **exclude** the abyssal id.
- R2: the friendly slot id is present in the result (coverage not regressed).
- R3: with `enemy_slot_border = None`, both ids are returned (no filtering) — current
  behavior preserved.
- R4 (regression guard): `item_up` resolution still includes/remaps as before for an
  abyssal id (the fall-through filter must not leak into the `item_up` branch).

  Fixture note (implementation-time): construct a minimal `ApiManifest` with two
  `api_mst_slotitem` entries straddling the border and a `CacheRulesAsset` with
  `enemy_slot_border` set, rather than relying on the real `start2.json`. If no
  lightweight `ApiManifest` builder exists in the test module, build one inline from
  `Default` plus the two pushed slot items.

**Verification:** `cargo test -p emukc_bootstrap` green; a fresh `cargo run -- cache
make-list` no longer emits `kcs2/resources/slot/{card,card_t,item_on,item_character,
statustop_item,remodel}/{1658,1659}_*.png`; friendly slot paths are unchanged
(spot-check a friendly id such as 575 still appears in its categories).

## Scope Boundaries

### In Scope
- Excluding abyssal equipment from standard slot categories via the existing
  `enemy_slot_border` (U1).

### Out of Scope
- The `banner_g_dmg` abyssal omission — already shipped in a prior change
  (decoder `SHIP_TARGET_SEMANTIC_CASES` + re-synced assets).
- Any decoder / `cache_rules.json` / asset regeneration — the border value already
  exists; this is a consumer-side fix.

### Deferred to Follow-Up Work
- **album_status 404s for ships 743/744/745** (`長波改二補` / `朝霜改二補` /
  `涼波改二補`). These friendly remodel/supply variants genuinely lack a 図鑑
  (picture-book) entry, so `album_status` has no resource. There is no clean
  boundary rule to exclude them — it needs a maintained hole-list analogous to
  `ENEMY_SHIP_HOLES`, which is brittle and game-update-sensitive. Only 3 entries,
  harmless (populate 404s and skips). Defer until it's worth maintaining a list.
- **Low-confidence "consumed but ungenerated" targets** `banner3_dmg` and
  `power_up_dmg`: referenced as client strings but have no manifest entry and were
  deliberately excluded from their base type's damage-variant lists. No runtime
  404 evidence that the resources exist; treat as dead/defensive client strings.
  Revisit only if a real `missing`→fetch log appears for them.

## Verification

```bash
cargo test -p emukc_bootstrap        # U1 regression + existing make-list tests
cargo fmt --all --check
cargo clippy --workspace -- -W warnings
cargo run -- cache make-list         # confirm 1658/1659 slot paths no longer listed
```
