---
title: "PathRules deserialization from resource manifest"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "Loading resource_manifest.json and exposing path rules to make-list generation"
  - "Migrating hardcoded path-rule constants to manifest-driven values"
tags: [pathrules, resource-manifest, deserialization, oncelock, backward-compat]
related_components: []
---

# PathRules deserialization from resource manifest

## Context

The `resource_manifest.json` version 2 format carries a `pathRules` block
that supersedes hardcoded Rust constants for ship/slot category and variant
lookups. This contract documents how the block is deserialized into a typed
`PathRules` struct, how it is exposed via static `OnceLock`s, and how v1
manifests remain backward-compatible.

## Guidance

**PathRules deserialization from v2 manifest.**

- Deserialize the `pathRules` block from `resource_manifest.json` version 2
  into a typed `PathRules` struct containing fields for all hardcoded
  constant categories:
  `shipDamageVariants` (HashMap), `shipStandardCategories` (Vec),
  `shipFullCategories` (Vec), `slotStandardCategories` (Vec),
  `enemyPlaneIds` (Vec), `btxtFlatSlotIds` (Vec), `characterHoleIds` (Vec),
  `eventShipHoles` (HashMap), `enemyShipHoles` (HashMap), `specialShips`
  (Vec), `spRemodelShips` (Vec), `spRemodelMes` (Vec), `cardRounds` (Vec),
  `rewardShips` (Vec).
- When a v2 manifest with a `pathRules` block is loaded, populate a
  `static PATH_RULES: OnceLock<PathRules>` for downstream access.
- When a v1 manifest (no `pathRules` block) is loaded, deserialization must
  succeed without error, `PATH_RULES` remains unpopulated, and downstream
  code falls back to hardcoded constants.
- When `pathRules` exists but some fields are omitted or empty arrays, those
  fields deserialize as empty collections (default); downstream code using
  them falls back to constants when the collection is empty.

**Backward-compatible ResourceManifest loading.**

- `ResourceManifest` must accept both v1 and v2 manifests. The `path_rules`
  field uses `#[serde(default)]` so v1 manifests deserialize with
  `path_rules: None`.
- Loading a v1 manifest: `ResourceManifest.path_rules` is `None`, no warning
  emitted.
- Loading a v2 manifest with `pathRules`: `ResourceManifest.path_rules` is
  `Some(PathRules { ... })`, and both `PATH_RULES` and `BTXT_FLAT_COVERAGE`
  OnceLocks are populated.

**BTXT_FLAT_COVERAGE OnceLock initialization.**

- When `pathRules.btxtFlatSlotIds` is present and non-empty, populate a
  `static BTXT_FLAT_COVERAGE: OnceLock<HashSet<i64>>` from those IDs.
- When a v1 manifest is loaded (no pathRules), `BTXT_FLAT_COVERAGE` remains
  uninitialized and `has_btxt_flat_coverage()` falls back to the
  `BTXT_FLAT_IDS` constant.

**pathRules access helper.**

- Provide a `pub(crate) fn path_rules() -> Option<&'static PathRules>` helper
  returning the contents of the `PATH_RULES` OnceLock.
- After a v2 manifest loads, `path_rules()` returns `Some(&PathRules)`.
- With no manifest loaded or a v1 manifest, `path_rules()` returns `None`.

## Why This Matters

The OnceLock-plus-constant fallback design lets the make-list generators
query a single source of truth without caring whether the manifest is v1 or
v2. Breaking the backward-compatible path (e.g. making `path_rules` required)
would silently change cache-list output for any manifest that has not been
re-extracted at v2.

## When to Apply

- When changing the `pathRules` schema or adding a new constant category.
- When debugging a cache-list output difference after a manifest version
  bump.

## Examples

- v2 manifest with `pathRules.btxtFlatSlotIds` of 336 IDs: `BTXT_FLAT_COVERAGE`
  initializes with that `HashSet<i64>`, and `has_btxt_flat_coverage()` queries
  it.
- v1 manifest loaded: `path_rules()` returns `None`, generators use
  hardcoded constants, output is identical to pre-pathRules behavior.

## Related

- `docs/solutions/best-practices/pathrules-makelist-integration.md`
- `docs/solutions/best-practices/resource-manifest.md`
