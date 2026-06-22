---
title: "Material (resource) management: categories, caps, deduction, and regeneration"
date: 2026-06-22
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Adding or modifying material add/deduct logic or caps"
  - "Implementing operations that consume materials (construction, crafting, resupply)"
  - "Wiring material regeneration on port entry"
tags: [material, resources, caps, regeneration, deduction, materialops]
related_components: [emukc_db, emukc_model]
---

# Material (resource) management: categories, caps, deduction, and regeneration

## Context

Materials are the 8 core resources tracked per profile. `MaterialOps`
(`emukc_gameplay`) governs representation, caps, atomic deduction,
regeneration, initialization, and SeaORM persistence under
`entity::profile::material`. Migrated from the retired openspec material capability spec (see `docs/migration/openspec-sunset-log.md`).

## Guidance

### Categories and representation

The system SHALL track 8 material categories: fuel, ammo, steel, bauxite,
instant repair (torch/bucket), instant construction (torch), development
material (devmat), and improvement material (screw). Retrieved via
`get_materials`, all 8 categories SHALL be present with non-negative integer
values.

### Caps

Each category SHALL have a maximum capacity determined by server configuration
and the player's HQ level, applied via Codex game config.

- Adding materials within the cap: the full amount is added.
- Adding materials that would exceed a category cap: the value is clamped to
  the cap (no overflow).
- `add_material_impl` SHALL call `apply_hard_cap` using Codex game
  configuration before completing.

### Deduction

Materials SHALL be deducted atomically for construction, crafting, resupply,
and other operations via `deduct_material_impl`. All requested categories MUST
have sufficient stock or none are deducted.

- Successful deduction: materials are deducted and the resulting state returned.
- Insufficient materials: the operation fails with an `Insufficient` error
  indicating the category, current stock, and requested amount; no materials
  are deducted (the check happens before any mutation).
- Zero or negative deduction amounts are skipped (no error, no mutation).

### Regeneration

Fuel, ammo, steel, and bauxite SHALL regenerate over time based on HQ level
and server configuration via `apply_self_replenish` on port entry
(`update_materials`). Regenerated values SHALL be clamped to the material cap
and the updated state persisted.

### Initialization

New profiles SHALL receive a starting set of materials from Codex game
configuration via `codex.game_cfg.material.new_material()`.

### Persistence

Material changes SHALL be persisted via the SeaORM material entity under
`entity::profile::material`. `_impl` functions that modify materials within a
transaction SHALL only commit when the enclosing transaction commits. Exactly
one material record SHALL exist per `profile_id` (enforced by the database).

## Why This Matters

Materials gate nearly every gameplay action (construction, crafting, resupply,
expedition rewards). Atomic deduction prevents partial-state corruption when a
multi-category operation is interrupted. Cap clamping preserves the resource
economy. Transactional `_impl` functions are what let public trait methods
compose material changes safely with other domain operations.

## When to Apply

- When implementing any feature that consumes or grants resources.
- When adding a `_impl` material helper that participates in a transaction.
- When adjusting regeneration rates or cap configuration in the Codex.

## Examples

- `deduct_material_impl(fuel=100, ammo=50)` on a profile with `fuel=200,
  ammo=40` fails with `Insufficient { category: ammo, current: 40, requested:
  50 }` and mutates nothing.
- `add_material_impl` over the cap clamps rather than overflows.
- `update_materials` on port entry runs `apply_self_replenish`, clamps to cap,
  persists.

## Related

- `docs/solutions/architecture-patterns/quest.md` — quest rewards route through
  `add_material_impl`.
- `docs/solutions/architecture-patterns/useitem-response.md` — special
  resources (bucket/torch/devmat/screw) are read from the material table.
