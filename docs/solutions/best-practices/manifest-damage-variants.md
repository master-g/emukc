---
title: "Ship damage variant path generation and the static variant mapping table"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "Generating ship resource paths with damage variants"
  - "Maintaining the base-to-variant mapping table"
  - "Interpreting the damagedSource manifest field"
tags: [manifest, ship, damage-variant, path-generation, static-mapping]
related_components: [emukc_cache]
---

# Ship damage variant path generation and the static variant mapping table

## Context

Ship resources come in damage variants (e.g. `banner`, `banner_dmg`,
`banner_g_dmg`, `banner_g`) that the client requests based on in-battle ship
state. The manifest gives a base `targetType` and an obfuscated/variable
`damagedSource` field whose value (`_0x1a3f79`, `"true"`, `"false"`,
`"damaged"`, …) is NOT a reliable signal on its own. The path generator must
map base target types to their known damage variants via a static table, and
must NOT trust the `damagedSource` string to decide variant expansion — doing
so produces wrong or missing paths.

## Guidance

### Damage variant path generation

The manifest-driven ship path generator SHALL produce damage variant paths
(`_dmg`, `_g_dmg`, `_g`) for base target types that have known variants, using a
**static mapping table** as the source of truth for which variants exist.

The generator SHALL interpret the manifest `damagedSource` field as follows,
per base type:

- **`banner` with a variable/obfuscated `damagedSource`** (e.g.
  `_0x1a3f79`) → produce `banner`, `banner_dmg`, `banner_g_dmg`, `banner_g`.
- **`full` with `damagedSource = "false"`** → produce ONLY the `full` path,
  NOT `full_dmg`.
- **`full` with `damagedSource = "true"`** → produce ONLY the `full_dmg` path
  (the damaged variant of the base type).
- **`character_full` with a variable `damagedSource`** (e.g. `"damaged"`) →
  produce BOTH `character_full` and `character_full_dmg`.
- **Base type with no damage variants in the mapping** (e.g. `album_status`) →
  produce ONLY the base path, regardless of `damagedSource`.

### Damage variant mapping table

The system SHALL maintain a static mapping from base ship target types to their
damage variant target types. Consulting the table:

- base `"banner"` → `["banner_dmg", "banner_g_dmg", "banner_g"]`.
- base `"card"` → `["card_dmg"]`.
- base type NOT in the table (e.g. `"special"`) → no variants (empty).

## Why This Matters

The `damagedSource` field is obfuscated and inconsistently typed across
manifest entries; treating it as a reliable boolean or selector produces both
false-positive variants (downloading resources that do not exist) and
false-negative omissions (missing resources the client will request and 404 on
in battle). The static table encodes the empirically-known variant families and
makes generation deterministic; the `damagedSource` rules above layer the
true/false distinction only where it is reliable (the `full` family).

## When to Apply

- When adding a new ship `targetType` — check whether it has damage variants and
  update the static table.
- When changing how `damagedSource` is interpreted — do not trust the raw
  string; route through the per-family rules above.
- When a client 404s on a `*_dmg`/`*_g` resource — verify the base type is in
  the table and the generator emitted the variant.

## Examples

A `banner` entry with `damagedSource = "_0x1a3f79"` expands to four paths via
the table. A `full` entry with `damagedSource = "true"` emits only `full_dmg`.
An `album_status` entry emits only `album_status` because the table has no
variants for it, no matter what `damagedSource` says.

## Related

- `cache-manifest-integration.md` — the decoder-driven path that can OVERRIDE
  legacy variant expansion when a decoder semantic rule covers the family.
