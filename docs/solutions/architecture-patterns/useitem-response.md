---
title: "Useitem response: special resources derived from the material table"
date: 2026-06-22
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: medium
applies_when:
  - "Modifying api_get_member/useitem or api_get_member/require_info responses"
  - "Adding operations that change bucket/torch/devmat/screw and must surface in useitem"
tags: [useitem, require-info, special-resources, material-table, api-response]
related_components: [emukc_db]
---

# Useitem response: special resources derived from the material table

## Context

The `api_get_member/useitem` and `api_get_member/require_info` endpoints expose
bucket, torch, devmat, and screw counts. These four special resources live in
the `material` table (managed by `MaterialOps`), not the `use_item` table.
Migrated from `openspec/specs/useitem-response/spec.md`.

## Guidance

### Special resources sourced from the material table

`api_get_member/useitem` and `api_get_member/require_info` SHALL return bucket,
torch, devmat, and screw counts sourced from the `material` table, not the
`use_item` table. Other use items SHALL continue to be sourced from the
`use_item` table.

- `api_id=1` (Bucket) → `api_count` equals current `material.bucket`.
- `api_id=2` (Torch) → `api_count` equals current `material.torch`.
- `api_id=3` (DevMat) → `api_count` equals current `material.devmat`.
- `api_id=4` (Screw) → `api_count` equals current `material.screw`.
- Response entries with other `api_id` values are sourced from the `use_item`
  table as before.
- When the `use_item` table has no record for these 4 items, the response SHALL
  still include them with counts from the material table.
- `api_get_member/require_info`'s `api_useitem` field SHALL include
  bucket/torch/devmat/screw entries with counts from the material table,
  consistent with the useitem endpoint.
- A consume-use-item exchange that grants a material (e.g., a medal consumed
  for buckets via `consume_use_item_impl`) SHALL be reflected in the Bucket
  `api_count` on the next `useitem` call, since the grant routes through the
  material table.

## Why This Matters

Material mutations (`add_material_impl` / `deduct_material_impl`) only touch
the `material` table. If the useitem response sourced these 4 resources from
`use_item`, the client would show stale counts after expeditions, construction,
or exchanges — a silent desync between server state and client display.

## When to Apply

- When modifying the useitem or require_info response handlers.
- When adding an operation that changes special resources and must be visible
  to the client.

## Examples

- An expedition granting `bucket+2` via `add_material_impl` → next `useitem`
  call shows `api_id=1` with the updated `material.bucket`.
- Instant construction consuming torch via `deduct_material_impl` → next
  `useitem` call shows `api_id=2` reflecting the deduction.
- A medal-for-bucket exchange via `consume_use_item_impl` → bucket count
  reflects the added buckets.

## Related

- `docs/solutions/architecture-patterns/material.md` — the material table is
  the source of truth for these 4 special resources.
