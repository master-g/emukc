---
title: "PathRules integration into make-list generation"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "Wiring pathRules values into generate, slot, and ship make-list functions"
  - "Validating output parity between pathRules-driven and constant-driven generation"
tags: [pathrules, make-list, generate, slot, ship, parity]
related_components: []
---

# PathRules integration into make-list generation

## Context

Once `pathRules` is loaded from the manifest (see
`pathrules-loading.md`), the make-list generators must consult it before
falling back to hardcoded constants. This contract documents which functions
consume which `pathRules` fields, and the output-parity guarantee that must
hold when pathRules values match the constants.

## Guidance

**generate.rs uses PathRules for category and variant lookups.**

- `generate_entry_paths()` in
  `crates/emukc_bootstrap/src/make_list/manifest/generate.rs` must check
  `path_rules()` for ship damage variants, standard categories, full
  categories, and slot categories. When `path_rules()` returns `Some`, those
  values replace the hardcoded `SHIP_DAMAGE_VARIANTS`,
  `SHIP_STANDARD_CATEGORIES`, `SHIP_FULL_CATEGORIES`, and
  `SLOT_STANDARD_CATEGORIES` constants.
- When `path_rules()` returns `Some(rules)` during Default/Greedy generation:
  damage variant lookups use `rules.ship_damage_variants`; category
  membership checks use `rules.ship_standard_categories`,
  `rules.ship_full_categories`, `rules.slot_standard_categories`; no
  hardcoded constants are consulted.
- When `path_rules()` returns `None`, `generate_entry_paths()` uses the
  existing hardcoded constants; output is identical to current behavior.

**slot.rs uses PathRules for coverage and generation.**

- Functions in
  `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/slot.rs` must
  check `path_rules()` for enemy plane IDs, btxt_flat slot IDs, and character
  hole IDs.
- When `path_rules()` returns `Some(rules)`: `make_enemy_plane()` uses
  `rules.enemy_plane_ids` instead of `ENEMY_PLANE_MAX_ID`;
  `make_btxt_flat()` uses `rules.btxt_flat_slot_ids` instead of
  `BTXT_FLAT_IDS`; `make_character()` uses `rules.character_hole_ids`
  instead of `CHARACTER_HOLES`.
- When `path_rules()` returns `None`, slot generation uses existing
  constants; output is identical to current behavior.

**ship.rs uses PathRules for hole and special ship lists.**

- Functions in
  `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/ship.rs` must
  check `path_rules()` for event ship holes, enemy ship holes, special ships,
  sp_remodel data, card rounds, and reward ships.
- When `path_rules()` returns `Some(rules)`: hole lookups use
  `rules.event_ship_holes`, `rules.enemy_ship_holes`; special ship checks
  use `rules.special_ships`; sp_remodel generation uses
  `rules.sp_remodel_ships`, `rules.sp_remodel_mes`; card/reward generation
  uses `rules.card_rounds`, `rules.reward_ships`.
- When `path_rules()` returns `None`, ship generation uses existing
  `LazyLock` constants; output is identical to current behavior.

**has_btxt_flat_coverage uses manifest-derived set.**

- `has_btxt_flat_coverage()` must check `BTXT_FLAT_COVERAGE` OnceLock first.
  If initialized, it queries the `HashSet<i64>`. If not initialized, it falls
  back to the `BTXT_FLAT_IDS` constant.
- With a v2 manifest loaded and `BTXT_FLAT_COVERAGE` populated,
  `has_btxt_flat_coverage(known_id)` returns `true` for IDs in the manifest
  set and `false` for IDs not in it.
- Without a v2 manifest, `has_btxt_flat_coverage()` returns the same result
  as `BTXT_FLAT_IDS.contains()`.

**Output parity validation.**

- Include a test verifying that Default strategy output with v2 `pathRules`
  produces the same resource paths as Default strategy output without
  `pathRules`.
- When `pathRules` fields are populated from the same game version as the
  hardcoded constants: Default strategy output with `pathRules` is identical
  to output with constants; Greedy strategy output with `pathRules` is
  identical to output with constants.
- When `pathRules` fields differ from hardcoded constants (game version
  mismatch): a test warning reports which fields differ and by how many
  entries; generation proceeds with `pathRules` values (not constants).

## Why This Matters

The parity test is the safety net for the pathRules migration: it proves the
manifest-driven path produces identical output when values match, and surfaces
a clear diff when they diverge (e.g. after a game update). Without it, a
stale manifest silently changes cache-list output.

## When to Apply

- When adding a new constant category to the make-list generators.
- After re-extracting the manifest at a new game version, to confirm parity
  or surface the expected diff.

## Examples

- v2 manifest with pathRules matching constants: Default and Greedy output are
  byte-identical to the constant-driven path; the parity test passes silently.
- Game update changes `characterHoleIds`: the parity test warns that the field
  differs by N entries; generation proceeds with the manifest values.

## Related

- `docs/solutions/best-practices/pathrules-loading.md`
- `docs/solutions/best-practices/resource-manifest.md`
