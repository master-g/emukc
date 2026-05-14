---
title: fix: Resolve code review findings on feat/vibe branch
type: fix
status: superseded
superseded-by: 2026-05-06-001
date: 2026-05-04
---

# fix: Resolve code review findings on feat/vibe branch

## Summary

Fix correctness, dead-code, and test-coverage issues found during structured code review of the wikiwiki route parser, kcdata merge pipeline, and map routing runtime changes on the `feat/vibe` branch.

---

## Requirements

- R1. No dead code in production paths (second ランダム block, master_cell_id strip)
- R2. Inferred deterministic edges in `build_nodes` must not silently skip non-combat intermediate nodes
- R3. All new code follows project formatting (hard_tabs = false → 4-space indent)
- R4. Comments accurately describe what the code does
- R5. New logic has dedicated unit test coverage (inferred edges, parse_node_labels, edge cases)

---

## Scope Boundaries

- Only fixes code already on the `feat/vibe` branch — no new features
- Does NOT redesign the kcdata merge pipeline or inferred-edge heuristic — only corrects and documents the current approach
- Behavioral change in `map_route.rs` SourceUnknown fallback (deterministic → random) is documented but not reverted — it is intentional

### Deferred to Follow-Up Work

- kcdata.rs (754 lines) full review — separate review pass
- Full resolution of 6-1/6-4/7-4 wikiwiki parsing gaps — per existing plan 2026-05-04-001
- CI check for zero Unknown predicates in catalog — future work

---

## Key Technical Decisions

- **Keep SourceUnknown behavioral change:** The old code picked `BTreeSet::first()` (deterministic minimum cell_no). New code calls `select_route_from_cells` (random from `next_cells`). This is intentional — SourceUnknown means we don't understand the real routing, so random selection from structural edges is more defensible than always picking the smallest cell number. Document in commit message.
- **Inferred edges use `all_nodes` not just `enemy_nodes`:** The current filter `enemy_nodes.contains_key(*lbl)` skips non-combat intermediate nodes (resource, whirlpool, air battle). Change to connect target-only nodes to the next node in BFS order regardless of type, preserving path continuity.
- **Remove master_cell_id strip entirely:** Both wikiwiki and kcdata parsers produce `master_cell_id: None`. The strip loop is a no-op. The comment claiming "overlay merge will assign correct mcid" is false — runtime uses fallback `map_id * 100 + cell_no`. Remove the dead code and fix the comment.

---

## Implementation Units

- U1. **Remove dead code and fix comments**

**Goal:** Eliminate unreachable ランダム block and dead master_cell_id strip loop. Fix misleading comment.

**Requirements:** R1, R4

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs` (lines 698-714: delete second ランダム block)
- Modify: `crates/emukc_bootstrap/src/map_pipeline/assemble.rs` (lines 22-33: remove master_cell_id strip, fix comment)

**Approach:**
- Delete the second ランダム block (route.rs:698-714). It is unreachable because `sanitize_route_text` converts `"ランダム\n..."` to `"ランダム ..."`, which is already caught by the first block at line 664.
- Remove the wikiwiki master_cell_id strip loop (assemble.rs:22-33). Replace the comment with an accurate one: wikiwiki does not set master_cell_id, kcdata does not set it, merged result relies on runtime fallback `map_id * 100 + cell_no`.

**Test expectation:** none — removing dead code does not change behavior.

**Verification:**
- `cargo test -p emukc_bootstrap` passes
- `cargo test -p emukc_gameplay` passes

---

- U2. **Fix inferred edges to include all intermediate nodes**

**Goal:** Inferred deterministic edges should connect to the next node in BFS order regardless of type, not skip non-combat nodes.

**Requirements:** R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs` (lines 420-427: change filter)

**Approach:**
- Change the filter at line 421-424 from `enemy_nodes.contains_key(*lbl)` to check if the label is in the `graph` (i.e., any node). This ensures resource nodes, whirlpool nodes, and air battle nodes are not skipped in inferred paths.
- Alternative: if non-combat nodes are never in the wikiwiki route table (and thus never in `graph`), the filter change is a no-op. Investigate first, then apply only if needed.

**Execution note:** Investigate `enemy_nodes` construction to determine if non-combat nodes are ever present in the graph before changing the filter.

**Test scenarios:**
- Unit test: graph with target-only node between two combat nodes → inferred edge connects to immediate next node, not second combat node
- Unit test: target-only boss node → no inferred edge (boss is excluded)

**Verification:**
- New test passes
- Existing tests still pass

---

- U3. **Fix indentation and run cargo fmt**

**Goal:** Normalize all changed code to project standard (4-space indent, `hard_tabs = false`).

**Requirements:** R3

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route.rs`
- Modify: `crates/emukc_gameplay/src/game/map_route.rs`

**Approach:**
- Run `cargo fmt --all` after U1 and U2 changes land
- Verify no formatting diff with `cargo fmt --check`

**Test expectation:** none — formatting only.

**Verification:**
- `cargo fmt --check` exits 0

---

- U4. **Add missing unit tests**

**Goal:** Cover new logic with targeted tests — inferred edges, `parse_node_labels`, edge cases for ランダム matching.

**Requirements:** R5

**Dependencies:** U2

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs`
- Modify: `crates/emukc_gameplay/src/game/map_route.rs` (inline `mod tests`)

**Approach:**

**wikiwiki_map/tests.rs:**
- Test `parse_node_labels`: empty input → empty vec; "A" → ["A"]; "A/B" → ["A","B"]; "スタート" → ["Start"]; "A/B/C" → ["A","B","C"]
- Test inferred edges: mock route_rules + enemy_nodes where a target-only node exists → verify `build_nodes` produces correct `next_cells`
- Edge case: "ランダムではない" (starts with ランダム but isn't random) → should NOT match the ランダム handler. Currently `starts_with("ランダム ")` after sanitization becomes `starts_with("ランダム ではない")` — verify this is handled correctly or add guard

**map_route.rs tests:**
- Test all-SourceUnknown with `selected_cell_id = Some(X)` where X is in rule targets but NOT in `next_cells`
- Test indeterminate fallback with `selected_cell_id = Some(X)` where X is NOT in `next_cells` → verify error, not panic

**Test scenarios:**
- Happy path: parse_node_labels splits "A/B" correctly
- Edge case: parse_node_labels with empty string returns empty vec
- Edge case: "ランダムではない" does not trigger ランダム handler
- Integration: inferred edges produce correct next_cells for a simple linear map
- Integration: SourceUnknown rules with selected_cell_id outside next_cells returns error

**Verification:**
- `cargo test -p emukc_bootstrap -- wikiwiki_map` passes
- `cargo test -p emukc_gameplay -- map_route` passes

---

## System-Wide Impact

- **Interaction graph:** U1 removes dead code only (no behavior change). U2 may change inferred edges in wikiwiki catalog, affecting `next_cells` for some map nodes. If non-combat nodes are currently skipped, this change adds them back to the path.
- **Unchanged invariants:** Runtime routing evaluation, merge logic, API response format — all unchanged.
- **Regression risk:** U2 (inferred edges change) could alter `cell_no` numbering if BFS traversal changes. Run full test suite including `repo_asset_limits_route_history_rules_to_known_normal_maps` after changes.

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| U2 may change cell_no numbering for existing maps | Run guardrail test `repo_asset_limits_route_history_rules_to_known_normal_maps` and update expected values if change is correct |
| Non-combat nodes may not exist in wikiwiki route table at all | Investigate first in U2 execution — if confirmed, skip the filter change |
| cargo fmt may touch unrelated files | Review fmt diff before committing; only commit changed lines |
