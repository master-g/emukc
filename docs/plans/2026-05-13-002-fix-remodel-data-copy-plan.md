---
name: fix-remodel-data-copy
status: active
created: 2026-05-13
---

# Fix: Remodel Data Copy Omissions

## Problem

`remodel_impl` creates a new `KcApiShip` via `codex.new_ship()` then selectively copies data from the old ship. Two fields are not properly preserved, causing data loss on remodel.

## Bugs

### Bug 1: `sally_area` reset to 0

- **Location**: `crates/emukc_gameplay/src/game/compose/remodel.rs:202-209`
- **Cause**: `sally_area` is not included in the `Unchanged` overrides. `From<KcApiShip> for ActiveModel` sets `sally_area: ActiveValue::Set(value.api_sally_area)` where `api_sally_area` is `0` from `new_ship()`, overwriting the DB value.
- **Fix**: Add `am.sally_area = ActiveValue::Unchanged(ship_model.sally_area)` alongside the other `Unchanged` lines.

### Bug 2: `ex_slot` item unequipped but not re-equipped

- **Location**: `crates/emukc_gameplay/src/game/compose/remodel.rs:71,184-188`
- **Cause**: `ship_model.slot_ex` is included in the undress loop (line 71), setting `equip_on = 0` on the item. But the remodel code then sets `new_ship.api_slot_ex` to a capacity marker (`-1` if old slot was nonzero, `0` otherwise) instead of the actual item ID. The item is orphaned — unequipped from old ship but never re-equipped on new ship.
- **Fix**: Remove `ship_model.slot_ex` from the undress loop, and set `new_ship.api_slot_ex = ship_model.slot_ex` to preserve the actual item reference.

## Implementation

### File: `crates/emukc_gameplay/src/game/compose/remodel.rs`

**Change 1** — Remove `slot_ex` from undress loop (line 71):

```rust
// Before:
for slot_item_id in [
    ship_model.slot_1,
    ship_model.slot_2,
    ship_model.slot_3,
    ship_model.slot_4,
    ship_model.slot_5,
    ship_model.slot_ex,  // <-- remove this
] {

// After:
for slot_item_id in [
    ship_model.slot_1,
    ship_model.slot_2,
    ship_model.slot_3,
    ship_model.slot_4,
    ship_model.slot_5,
] {
```

**Change 2** — Preserve `slot_ex` item ID (lines 184-188):

```rust
// Before:
new_ship.api_slot_ex = if ship_model.slot_ex != 0 {
    -1
} else {
    0
};

// After:
new_ship.api_slot_ex = ship_model.slot_ex;
```

**Change 3** — Preserve `sally_area` (after line 207):

```rust
// Add:
am.sally_area = ActiveValue::Unchanged(ship_model.sally_area);
```

## Test Scenarios

### File: `tests/gameplay_tests/`

Add test `test_remodel_preserves_sally_area_and_ex_slot`:

1. Create a ship with `sally_area > 0` and an item equipped in `slot_ex`.
2. Run remodel.
3. Assert `sally_area` unchanged after remodel.
4. Assert `slot_ex` still references the same item ID.
5. Assert the item's `equip_on` still points to the ship.

## Verified Non-Issues

- `condition` reset to 40: Wiki confirms remodel resets morale. Correct behavior.
- `fuel`/`ammo` reset to max: User confirmed this is intended.
- `married` preserved via `ActiveValue::NotSet` in `From<KcApiShip> for ActiveModel`.
