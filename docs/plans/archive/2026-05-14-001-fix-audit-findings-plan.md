---
title: "fix: Correct boiler query logic and CLAUDE.md indent documentation"
type: fix
status: completed
date: 2026-05-14
---

# fix: Correct boiler query logic and CLAUDE.md indent documentation

## Summary

Fix two issues found during commit audit: (1) `get_free_slot_item_by_type3_impl` queries equipped items (`EquipOn.gt(0)`) when it should query unequipped items (`EquipOn.lte(0)`) — this means remodel consumes equipped boilers instead of free ones; (2) CLAUDE.md claims "Hard tabs for indentation" but `.rustfmt.toml` uses `hard_tabs = false`.

---

## Problem Frame

The boiler deduction during remodel silently consumes boilers that are currently equipped on other ships, which is incorrect game behavior. The CLAUDE.md documentation mismatch causes confusion for contributors about the project's actual indent style.

---

## Requirements

- R1. `get_free_slot_item_by_type3_impl` must return only unequipped, unlocked items of the given type3
- R2. CLAUDE.md must accurately reflect the project's indent style (`hard_tabs = false`, 4 spaces)

---

## Scope Boundaries

- Only fix the query filter and the doc line. No other remodel logic changes.
- The function name `get_free_slot_item_by_type3` is already correct for the intended semantics — no rename needed.

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_gameplay/src/game/slot_item.rs:380` — `get_unset_slot_items_impl` uses `EquipOn.lte(0)` for "unequipped" semantics (correct pattern)
- `crates/emukc_gameplay/src/game/slot_item.rs:397` — same pattern in `get_unset_slot_items_by_types_impl`
- `crates/emukc_gameplay/src/game/compose/remodel.rs:221` — the buggy line uses `EquipOn.gt(0)`

---

## Key Technical Decisions

- Use `EquipOn.lte(0)` (not `.eq(0)`) to match the existing `get_unset_slot_items_impl` pattern — `equip_on` is 0 when unequipped, and the `lte` guard is defensive against any negative sentinel values.

---

## Implementation Units

### U1. Fix boiler query filter

**Goal:** Make `get_free_slot_item_by_type3_impl` return unequipped items.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_gameplay/src/game/compose/remodel.rs`
- Test: `tests/gameplay_tests/remodel_preserve_fields.rs` (extend existing test or add new)

**Approach:**
- Change `.filter(slot_item::Column::EquipOn.gt(0))` to `.filter(slot_item::Column::EquipOn.lte(0))`

**Patterns to follow:**
- `crates/emukc_gameplay/src/game/slot_item.rs:380` — identical filter for "unequipped" semantics

**Test scenarios:**
- Happy path: remodel with 1 free (unequipped) boiler in inventory succeeds and consumes it
- Edge case: boiler exists but is equipped on another ship — remodel should fail with Insufficient error
- Edge case: multiple free boilers, lowest level consumed first (existing `order_by_asc(Level)`)

**Verification:**
- `cargo test --test gameplay_tests remodel` passes
- `cargo clippy --workspace` clean

### U2. Fix CLAUDE.md indent documentation

**Goal:** Correct the misleading "Hard tabs" claim.

**Requirements:** R2

**Dependencies:** None

**Files:**
- Modify: `CLAUDE.md`

**Approach:**
- Change "**Hard tabs** for indentation" to "**Soft tabs** (4 spaces) for indentation" to match `.rustfmt.toml` (`hard_tabs = false`) and `.editorconfig` (`indent_style = space`)

**Test expectation:** none — documentation-only change

**Verification:**
- CLAUDE.md text matches `.rustfmt.toml` and `.editorconfig`

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Existing remodel tests may rely on the buggy behavior (equipped boilers being consumed) | Review test setup — current tests grant free items, so the fix should not break them |
