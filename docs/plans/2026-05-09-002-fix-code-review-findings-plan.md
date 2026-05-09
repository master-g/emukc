---
title: "fix: address code review findings from route-based cell numbering"
type: fix
status: active
date: 2026-05-09
origin: Code review of uncommitted changes in feat/vibe branch
---

# fix: address code review findings from route-based cell numbering

## Summary

Fix formatting issues, remove unnecessary thread wrapper, add missing test coverage, and extract magic numbers from the route-based kcdata cell numbering implementation.

---

## Problem Frame

Code review of the route-based cell numbering changes identified 9 findings: 3 safe_auto formatting/cleanup issues, 5 manual test coverage gaps, and 1 advisory finding. The changes are functionally correct but need test hardening and maintainability improvements before merge.

---

## Requirements

- R1. Fix indentation to use soft tabs per `.editorconfig` and `.rustfmt.toml`
- R2. Remove unnecessary `thread::scope` wrapper in `download_stat_json`
- R3. Add test coverage for edge cases in route-based cell numbering
- R4. Extract magic numbers for cell colors/events into named constants
- R5. Verify no regressions in existing test suite

---

## Scope Boundaries

- Integration tests verifying end-to-end map loading with new numbering are out of scope (noted as residual risk)
- Circular route detection is out of scope (noted as residual risk)
- Multiple boss routes behavior is out of scope (noted as residual risk)

---

## Context & Research

### Code Review Findings

**P1 findings:**
- Missing test for route with None target_key (line 129)

**P2 findings:**
- Missing test for boss_cell_no=0 when no boss exists (line 126)
- Missing test for routes with no target node metadata (line 132)
- Missing test for empty routes map (line 125)
- Magic numbers (5,5,1), (4,4,1), (6,1,0) lack explanation (line 136)
- Unnecessary thread::scope wrapper (sources.rs:203)
- Hard tabs instead of soft tabs (kcdata.rs:117, sources.rs:203)

**P3 findings:**
- Thread panic in download_stat_json not tested (advisory)

### Relevant Code

- `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` - route-based cell builder
- `crates/emukc_bootstrap/src/map_pipeline/sources.rs` - stat.json downloader
- `.rustfmt.toml` and `.editorconfig` - formatting config

---

## Key Technical Decisions

- **Magic number extraction**: Define constants at module level with doc comments explaining game semantics
- **Test organization**: Add new test cases to existing test module rather than separate file
- **Thread::scope removal**: Direct tokio runtime usage is simpler and equivalent for single-spawn case

---

## Implementation Units

### U1. Fix formatting (run cargo fmt)

**Goal:** Convert hard tabs to soft tabs per project standards

**Requirements:** R1

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`
- Modify: `crates/emukc_bootstrap/src/map_pipeline/sources.rs`

**Approach:**
- Run `cargo fmt --all` to apply project formatting rules
- Verify `.rustfmt.toml` specifies soft tabs (hard_tabs = false)

**Patterns to follow:**
- Existing formatted Rust code in the repo

**Test scenarios:**
- Test expectation: none -- formatting change only

**Verification:**
- `cargo fmt --all --check` passes
- No diff after running `cargo fmt --all`

---

### U2. Remove thread::scope wrapper

**Goal:** Simplify `download_stat_json` by removing unnecessary thread indirection

**Requirements:** R2

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/sources.rs`

**Approach:**
- Remove `std::thread::scope` and `s.spawn` wrapper
- Keep tokio runtime creation and async block as-is
- The runtime already handles async execution; extra thread adds no value

**Patterns to follow:**
- Other tokio runtime usage in the codebase

**Test scenarios:**
- Happy path: stat.json download succeeds, returns content
- Error path: HTTP error returns Err with status code
- Error path: network timeout returns Err with timeout message

**Verification:**
- `cargo test -p emukc_bootstrap` passes
- Manual: `cargo run -- bootstrap` still downloads stat.json successfully

---

### U3. Extract magic numbers to named constants

**Goal:** Replace hardcoded cell color/event tuples with documented constants

**Requirements:** R4

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`

**Approach:**
- Define module-level constants before `build_variant_from_kcdata`:
  ```rust
  /// Boss cell appearance (red, battle node)
  const BOSS_CELL: (i64, i64, i64) = (5, 5, 1);
  /// Battle cell appearance (red, non-boss battle)
  const BATTLE_CELL: (i64, i64, i64) = (4, 4, 1);
  /// Empty/resource cell appearance (blue, no battle)
  const EMPTY_CELL: (i64, i64, i64) = (6, 1, 0);
  ```
- Replace inline tuples at line 136 with constant references
- Tuple fields are (color_no, event_id, event_kind)

**Patterns to follow:**
- Existing constant definitions in emukc_model

**Test scenarios:**
- Test expectation: none -- refactor only, behavior unchanged

**Verification:**
- `cargo test -p emukc_bootstrap kcdata` passes
- No change in test output

---

### U4. Add test for empty routes map

**Goal:** Verify behavior when `data.routes` is empty

**Requirements:** R3

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (test module)

**Approach:**
- Add test `build_variant_from_kcdata_handles_empty_routes`
- Construct `KcDataMapData` with empty routes map
- Assert `variant.cells` is empty
- Assert `boss_cell_no` is 0

**Test scenarios:**
- Edge case: empty routes map produces empty cells vector and boss_cell_no=0

**Verification:**
- `cargo test -p emukc_bootstrap build_variant_from_kcdata_handles_empty_routes` passes

---

### U5. Add test for routes with no target node metadata

**Goal:** Verify cells get default values when target node has no metadata entry

**Requirements:** R3

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (test module)

**Approach:**
- Add test `build_variant_from_kcdata_handles_missing_node_metadata`
- Construct route targeting node "X" with no entry in `data.cells`
- Assert cell gets default values: color_no=6, event_id=1, event_kind=0
- Assert node_label is set to target key "X"

**Test scenarios:**
- Edge case: route targeting unmapped node gets EMPTY_CELL defaults and target key as label

**Verification:**
- `cargo test -p emukc_bootstrap build_variant_from_kcdata_handles_missing_node_metadata` passes

---

### U6. Add test for boss_cell_no=0 when no boss exists

**Goal:** Verify `boss_cell_no` remains 0 when no routes target boss cells

**Requirements:** R3

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (test module)

**Approach:**
- Add test `build_variant_from_kcdata_boss_cell_no_zero_when_no_boss`
- Construct routes with cells that have `boss: false` or no metadata
- Assert `boss_cell_no` is 0

**Test scenarios:**
- Edge case: map with no boss cells leaves boss_cell_no at initial value 0

**Verification:**
- `cargo test -p emukc_bootstrap build_variant_from_kcdata_boss_cell_no_zero_when_no_boss` passes

---

### U7. Add test for route with None target_key (if possible)

**Goal:** Document whether `route_node_key` can return None and test if so

**Requirements:** R3

**Dependencies:** U1

**Files:**
- Modify: `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs` (test module or comment)

**Approach:**
- Inspect `KcDataNode` enum definition
- If `KcDataNode` is `Int | String`, `route_node_key` always returns Some
- If so, add comment explaining why target_key can never be None
- If None is possible, add test verifying cell creation handles it gracefully

**Test scenarios:**
- Edge case: if route.to can be None-producing, verify cell creation doesn't panic
- Or: document why None is impossible

**Verification:**
- Either test passes or comment added explaining impossibility

---

## System-Wide Impact

- **Unchanged invariants:** Route-based cell numbering semantics unchanged; only test coverage and code clarity improved
- **Integration coverage:** Existing integration tests in `tests/gameplay_tests/` verify end-to-end behavior

---

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| Formatting changes conflict with uncommitted work | Run `cargo fmt` before other changes; commit separately |
| Test additions reveal actual bugs | Good — fix bugs before merge |
| Magic number extraction changes behavior | Use const tuples with same values; verify tests pass |

---

## Sources & References

- Code review findings from `/compound-engineering:ce-code-review` on feat/vibe branch
- `.rustfmt.toml` and `.editorconfig` for formatting standards
- Existing test patterns in `crates/emukc_bootstrap/src/map_pipeline/kcdata.rs`
