---
title: Fix unused re-export clippy warnings in wikiwiki route module
type: fix
status: completed
date: 2026-05-07
---

# Fix unused re-export clippy warnings in wikiwiki route module

## Summary

Gate test-only re-exports with `#[cfg(test)]` and remove dead `unknown_predicate` re-export in `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/mod.rs`. Eliminates 9 clippy `unused_imports` warnings on non-test builds.

---

## Problem Frame

`cargo clippy -p emukc_bootstrap --lib` produces two `unused_imports` warnings in `route/mod.rs`: 7 unused re-exports from `route_condition` and 2 from `route_predicate`. The committed re-exports were blanket `pub(super) use` from the module split, but not all are consumed in production — some are used only by `tests.rs` via the `use super::*` → `use route::*` chain, and one (`unknown_predicate`) is never consumed through the re-export at all (it is imported directly by `route_condition.rs`).

---

## Requirements

- R1. Non-test (`--lib`) builds must produce zero clippy `unused_imports` warnings from `route/mod.rs`
- R2. All existing tests in `tests.rs` continue to compile and pass — test-only re-exports remain accessible via `#[cfg(test)]`

---

## Scope Boundaries

- Only `route/mod.rs` re-export blocks are modified
- No changes to function definitions, sub-module imports, or test file imports
- The 2 pre-existing fixture parser test failures are out of scope (predate the module split)

---

## Context & Research

### Relevant Code and Patterns

- `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/mod.rs` — the re-export file (94 lines)
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/mod.rs:25` — `use route::*;` consumes production re-exports
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/tests.rs:1` — `use super::*;` consumes test re-exports
- `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/route_condition.rs:15` — imports `parse_route_predicate` and `unknown_predicate` directly (bypasses the re-export)

### Usage audit

| Re-export | Consumer | Gate needed |
|---|---|---|
| `build_nodes` | `mod.rs` (production) | none |
| `check_mixed_routing_encoding` | `mod.rs` (production) | none |
| `parse_route_table` | `mod.rs` (production) | none |
| `postprocess_route_probabilities` | `mod.rs` (production) | none |
| `compact_route_raw_text` | `mod.rs` (production) | none |
| `collect_formations` | `mod.rs` (production) | none |
| `find_route_table_sections` | `mod.rs` (production) | none |
| `parse_gauge_defeat_counts` | `mod.rs` (production) | none |
| `route_section_variant_key` | `mod.rs` (production) | none |
| `parse_case_route_condition_text` | `tests.rs` only | `#[cfg(test)]` |
| `parse_conditional_random_route_condition_text` | `tests.rs` only | `#[cfg(test)]` |
| `parse_independent_route_condition_line` | `tests.rs` only | `#[cfg(test)]` |
| `parse_inline_targeted_route_condition_text` | `tests.rs` only | `#[cfg(test)]` |
| `parse_row_target_random_bias_condition_text` | `tests.rs` only | `#[cfg(test)]` |
| `parse_row_target_random_bias_shorthand_condition_text` | `tests.rs` only | `#[cfg(test)]` |
| `parse_target_random_route_condition_text` | `tests.rs` only | `#[cfg(test)]` |
| `parse_route_predicate` | `tests.rs` only | `#[cfg(test)]` |
| `unknown_predicate` | **none via re-export** | **remove** |

---

## Implementation Units

- U1. **Gate test-only re-exports and remove unused `unknown_predicate`**

**Goal:** Split `route/mod.rs` re-exports into unconditional production block and `#[cfg(test)]` test block; drop `unknown_predicate` entirely from the re-export

**Requirements:** R1, R2

**Dependencies:** None

**Files:**
- Modify: `crates/emukc_bootstrap/src/parser/wikiwiki_map/route/mod.rs`

**Approach:**
1. Replace the two `pub(super) use` blocks (lines 5-19) with three blocks:
   - Unconditional block for 9 production re-exports (no gate)
   - `#[cfg(test)]` block for 7 test-only `parse_*` functions from `route_condition`
   - `#[cfg(test)]` block for `parse_route_predicate` from `route_predicate`
2. Remove `unknown_predicate` entirely — it is imported directly by `route_condition.rs`, not through the re-export

**Patterns to follow:**
- The file already uses `#[cfg(test)]` gating on lines 22-23 for `use super::RouteRuleDraft` — same pattern

**Test scenarios:**
- Happy path: `cargo clippy -p emukc_bootstrap --lib` produces zero warnings from `route/mod.rs`
- Happy path: `cargo test -p emukc_bootstrap --lib` — all existing tests pass (57 pass, 2 pre-existing failures unchanged)
- Edge case: `cargo check -p emukc_bootstrap` — production code still compiles (parent `mod.rs` re-exports unchanged)

**Verification:**
- `cargo clippy -p emukc_bootstrap --lib` reports no warnings from `route/mod.rs`
- `cargo test -p emukc_bootstrap` — no regressions in test count

---

## Sources & References

- Related plan: `docs/plans/2026-05-07-001-fix-code-review-findings-map-refactor-plan.md` (prior code review findings; U1-U6, of which U1-U3 and U5-U6 are implemented)
