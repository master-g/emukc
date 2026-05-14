---
title: "Fix kcdata parser test gaps: BATTLE_CELL assertion and parallel edge dedup"
type: fix
status: superseded
date: 2026-05-11
---

# Fix kcdata parser test gaps: BATTLE_CELL assertion and parallel edge dedup

> Superseded by `docs/plans/2026-05-11-003-fix-kcdata-route-cell-api-parity-plan.md`.
> This plan was written against the rejected unique-node parser direction.
> Route-cell API identity is now the source of truth; battle-cell and parallel
> route coverage are handled in the route-cell parser tests from that plan.

## Summary

Add missing test coverage for the refactored `build_variant_from_kcdata`: (1) direct assertion that cells with battle metadata produce `BATTLE_CELL` appearance (color_no=4, event_id=4, event_kind=1), and (2) verification that parallel routes sharing the same source and target produce deduplicated `next_cells`.

---

## Problem Frame

Code review of the kcdata cell-topology refactor (plan 001) identified two test gaps:
- `BATTLE_CELL` (color_no=4) is the most common cell type but no test directly asserts its fields. Tests only assert `BOSS_CELL` (color_no=5) and `EMPTY_CELL` (color_no=6).
- The `next_cells` deduplication logic (`if !source_cell.next_cells.contains(&target_no)`) has no test case with parallel edges.

---

## Requirements

- R1. At least one test directly asserts `color_no == 4`, `event_id == 4`, `event_kind == 1` for a non-boss cell with battle metadata.
- R2. At least one test verifies that two routes from the same source to the same target produce only one entry in `next_cells`.

---

## Scope Boundaries

- **In scope**: `kcdata.rs` inline tests only.
- **Out of scope**: verify.rs, assemble.rs, any runtime code changes.

---

## Implementation Units

### U1. Add BATTLE_CELL direct assertion test

**Goal:** Assert that a cell derived from kcdata metadata with `boss: false` and a non-empty name produces `BATTLE_CELL` appearance.

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (inline tests)

**Approach:**
- Add a test that creates a kcdata YAML with a named non-boss cell, builds the variant, and asserts `color_no == 4`, `event_id == 4`, `event_kind == 1` on that cell.
- Can reuse the existing linear-map test fixture or create a minimal new fixture.

**Patterns to follow:**
- Existing inline test structure using YAML literals and `parse_kcdata_map`.

**Test scenarios:**
- Happy path: cell with `name: "battle", boss: false` → color_no=4, event_id=4, event_kind=1.

**Verification:**
- `cargo test -p emukc_bootstrap -- kcdata` passes including the new test.

---

### U2. Add parallel edge dedup test

**Goal:** Verify that two routes from the same source to the same target produce only one entry in the source cell's `next_cells`.

**Requirements:** R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (inline tests)

**Approach:**
- Add a test with YAML containing two routes from A→B (e.g., route 2 and route 4 both from A to B with different labels or same labels). Build the variant, find cell A, assert `next_cells` contains B's cell_no exactly once.
- The kcdata YAML supports this via two route entries with the same `from` and `to` but different route IDs.

**Patterns to follow:**
- Existing inline test structure.

**Test scenarios:**
- Happy path: routes 2 and 4 both go from A to B → A's next_cells has B's cell_no once, not twice.

**Verification:**
- `cargo test -p emukc_bootstrap -- kcdata` passes including the new test.
