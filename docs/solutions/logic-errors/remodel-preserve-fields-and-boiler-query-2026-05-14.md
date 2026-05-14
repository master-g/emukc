---
title: "Remodel loses sally_area, ex-slot equipment, and consumes wrong boilers"
date: 2026-05-14
category: logic-errors
module: emukc_gameplay
problem_type: logic_error
component: service_object
symptoms:
  - Ship sally_area resets to 0 after remodel
  - Ex-slot enhancement item unequipped/lost after remodel (api_slot_ex becomes -1)
  - Remodel consumes boilers equipped on other ships instead of free inventory items
root_cause: logic_error
resolution_type: code_fix
severity: high
tags: [remodel, seaorm, activevalue, slot-ex, sally-area, equipon, boiler]
---

# Remodel loses sally_area, ex-slot equipment, and consumes wrong boilers

## Problem

Ship remodel (改造) in EmuKC had three data-loss bugs: `sally_area` was silently reset to 0, ex-slot enhancement equipment was stripped and replaced with a sentinel value, and boiler consumption for certain remodels targeted equipped items on other ships instead of free inventory items. All three caused irreversible player data loss during a core gameplay action.

## Symptoms

- Ship sortie area restriction (`sally_area`) cleared after remodel, allowing ships to sortie to maps they shouldn't access
- Ex-slot equipment (e.g., reinforcement expansion + equipped item) disappeared after remodel — `api_slot_ex` set to `-1` (capacity marker) instead of actual item ID
- Boilers equipped on other ships were consumed during remodel, silently stripping equipment from unrelated ships

## What Didn't Work

- The `api_slot_ex` fix initially appeared to be a simple assignment, but has an ordering dependency: the item ID must be assigned **after** `cal_ship_status()` runs. Setting it before causes lookup failures because `cal_ship_status` traverses slots and validates instance IDs against the new equipment list (session history).
- The investigation spent time verifying that `condition` (morale) resetting to 40 and `fuel`/`ammo` being refilled were NOT bugs — both behaviors are correct per KanColle wiki. This was necessary due diligence but consumed investigation effort (session history).

## Solution

### Fix 1: Preserve sally_area and ex-slot across remodel

**File:** `crates/emukc_gameplay/src/game/compose/remodel.rs`

Three changes:

**A — Remove slot_ex from undress loop:**

Before:
```rust
for slot_item_id in [
    ship_model.slot_1,
    ship_model.slot_2,
    ship_model.slot_3,
    ship_model.slot_4,
    ship_model.slot_5,
    ship_model.slot_ex,  // BUG: ex-slot item gets unequipped
] {
```

After:
```rust
for slot_item_id in [
    ship_model.slot_1,
    ship_model.slot_2,
    ship_model.slot_3,
    ship_model.slot_4,
    ship_model.slot_5,
    // slot_ex removed — stays equipped
] {
```

**B — Set api_slot_ex to actual item ID (after cal_ship_status):**

Before:
```rust
new_ship.api_slot_ex = if ship_model.slot_ex != 0 {
    -1  // sentinel: "has expansion but no item"
} else {
    0
};
```

After:
```rust
// Must be after cal_ship_status() — ordering dependency
codex.cal_ship_status(&mut new_ship, &new_slot_items, ship_model.married)?;
new_ship.api_nowhp = new_ship.api_maxhp;
new_ship.api_slot_ex = ship_model.slot_ex;  // actual item ID
```

**C — Add sally_area to Unchanged overrides:**

Before:
```rust
am.profile_id = ActiveValue::Unchanged(profile_id);
am.locked = ActiveValue::Unchanged(ship_model.locked);
am.has_locked_euqip = ActiveValue::Unchanged(ship_model.has_locked_euqip);
// sally_area missing!
```

After:
```rust
am.profile_id = ActiveValue::Unchanged(profile_id);
am.locked = ActiveValue::Unchanged(ship_model.locked);
am.has_locked_euqip = ActiveValue::Unchanged(ship_model.has_locked_euqip);
am.sally_area = ActiveValue::Unchanged(ship_model.sally_area);
```

### Fix 2: Query unequipped boilers instead of equipped ones

**File:** `crates/emukc_gameplay/src/game/compose/remodel.rs`

Before:
```rust
.filter(slot_item::Column::EquipOn.gt(0))   // matches EQUIPPED items
```

After:
```rust
.filter(slot_item::Column::EquipOn.lte(0))  // matches UNEQUIPPED items
```

The correct pattern already existed in `get_unset_slot_items_impl` at `slot_item.rs:380`, which uses `EquipOn.lte(0)`.

## Why This Works

**sally_area:** SeaORM's `ActiveModel` only skips fields marked `ActiveValue::Unchanged` during UPDATE. The `From<KcApiShip>` conversion fills all fields with default values. Without the `Unchanged` override, the ORM overwrites the DB value with the default (0).

**ex-slot:** The undress loop (`slot_1` through `slot_5`) removes regular equipment before remodel. Ex-slot is a structural slot (reinforcement expansion), not regular equipment — it should survive remodel intact. The original code set `api_slot_ex` to a sentinel `-1` (meaning "has expansion capacity") instead of the actual equipped item ID, causing the client to show an empty slot.

**boiler query:** `EquipOn` stores the ship ID an item is equipped on. `gt(0)` matches items currently on ships; `lte(0)` matches free inventory items. The function name `get_free_slot_item_by_type3` clearly intends the latter. Using `lte(0)` instead of `eq(0)` is defensive — it also catches any negative sentinel values.

## Prevention

- **ActiveModel field audit:** After any `From<X> for ActiveModel` conversion followed by `.update()`, list every DB column that should survive the update. Any missing `ActiveValue::Unchanged` will silently overwrite with defaults. Add a comment block enumerating preserved fields.
- **Cross-reference existing patterns:** Before writing a new equipment query, grep for existing queries on the same column. The correct `EquipOn.lte(0)` pattern already existed in `get_unset_slot_items_impl` — the buggy query was a reinvention.
- **Regression tests:** Both fixes have dedicated tests:
  - `tests/gameplay_tests/remodel_preserve_fields.rs` — verifies sally_area and ex-slot preservation
  - Inline tests in `remodel.rs` — verify boiler query excludes equipped items, excludes locked items, orders by level
- **Code review checklist for remodel-adjacent changes:** (a) Does the undress loop only cover regular slots? (b) Are all preserved fields in the Unchanged override set? (c) Do consume-item queries use the correct EquipOn condition?

## Related Issues

- Plan doc: `docs/plans/2026-05-13-002-fix-remodel-data-copy-plan.md` (sally_area + ex-slot investigation)
- Plan doc: `docs/plans/2026-05-14-001-fix-audit-findings-plan.md` (boiler query audit finding)
- Commits: `f942bbc` (preserve sally_area + ex-slot), `bc36cb3` (fix boiler query)
