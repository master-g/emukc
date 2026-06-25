---
title: "Bootstrap-side validators run over emukc_model public types; behavioral tests live in-crate"
date: 2026-06-25
category: architecture-patterns
module: emukc_bootstrap
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Adding a new validator / linter / cross-check inside emukc_bootstrap"
  - "Tempted to call a gameplay router or gameplay logic from emukc_bootstrap"
  - "Deciding where the behavioral test for a bootstrap validator should live"
tags: [emukc_bootstrap, validator, dependency-direction, layering, structural-vs-semantic, in-crate-test]
related_components: [emukc_model, emukc_gameplay]
---

# Bootstrap-side validators run over emukc_model public types; behavioral tests live in-crate

## Context

`emukc_bootstrap` carries a family of validators that mirror
`crate::battle_rules`'s validator / finding / report / severity shape:
`map_route_rules.rs` (structural map route validator) and
`source_crosscheck.rs` (wikiwiki vs `real_map_start_data` consistency linter).
Both were constrained by the same hard layering rule, and both solved it the
same way. Capture that once so the next bootstrap-side validator follows it
without re-discovering it.

## Guidance

### The dependency direction is one-way: bootstrap must not depend on gameplay

`emukc_gameplay` depends on `emukc_bootstrap`. The reverse is a dependency
cycle and is forbidden. Therefore a bootstrap-side validator **cannot call the
production gameplay router** (e.g. `evaluate_route_destination`) or any other
gameplay logic. Build the validator over the **public types from
`emukc_model`** instead (`MapVariantDefinition` / `MapStageDefinition`,
`MapCellDefinition`, `RouteRule`, `RoutePredicate`, `MapCatalog`,
`MapDefinition`). It re-checks the declared data, not the runtime behavior.

### Structural, not semantic (avoid checking a source against itself)

These validators catch **structural corruption** (route edges pointing off the
topology, missing cells, unsupported predicates, cell-set / boss-cell
divergence between sources) — not **semantic** correctness. Do not assert a
predicate threshold (e.g. `FleetSize >= 4` vs `>= 5`): that threshold comes
from the same wikiwiki source being validated, so checking it against itself is
circular. Do not assert a deterministic next-cell: routing legitimately uses
weighted random. `source_crosscheck` stays scope-honest — it is a consistency
linter over the **thin surface the two sources actually share** (per-map
cell-number sets and boss-cell identity via `api_bosscell_no`), because
`real_map_start_data` carries no routing edges or per-cell enemy fleets.

### Behavioral edge-legality tests live in-crate in emukc_gameplay

The behavioral counterpart — driving the *real* `evaluate_route_destination`
over a fleet-config matrix and asserting every returned cell is a declared
`next_cell` — requires gameplay, so it lives in an in-crate `#[cfg(test)]`
module in `crates/emukc_gameplay/src/game/map_route.rs`. It must be in-crate
(not an external `tests/` integration test) because the production router
internals it exercises are `pub(crate)`. Structural validation (bootstrap) and
behavioral validation (gameplay in-crate test) are deliberately split across
the two crates by the dependency direction.

## Why This Matters

Reaching "up" from emukc_bootstrap into emukc_gameplay to reuse the router
looks convenient but introduces a cycle the workspace forbids — it won't
compile, and the fix-by-duplication tempts a second copy of routing logic.
Keeping bootstrap validators over public model types, and parking the
behavioral test in-crate where the `pub(crate)` router lives, is the layout
that compiles, avoids logic duplication, and keeps each check at the layer that
owns its inputs.

## When to Apply

- Adding any validator / linter / cross-check to emukc_bootstrap: build it over
  emukc_model public types; mirror the battle_rules finding/report/severity
  shape.
- When the check needs real gameplay behavior: write it as an in-crate
  `#[cfg(test)]` test in emukc_gameplay, not in bootstrap and not in external
  `tests/`.
- When scoping a source cross-check: restrict to the surface both sources share;
  do not validate a source's thresholds against itself.

## Related

- `docs/solutions/architecture-patterns/battle-protocol-validator-boundary.md`
  — the validator/finding/report shape these mirror, and the protocol-vs-
  behavioral boundary on the battle side.
- `docs/solutions/architecture-patterns/map-data-authority.md` — the map
  catalog data these validators check, and its source merge authority.
