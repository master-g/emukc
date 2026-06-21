---
title: "Decoder-driven cache-list generation: manifest/category/coverage/rules integration"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: high
applies_when:
  - "Generating cache-list paths from decoder-produced manifest, category, coverage, or rules assets"
  - "Implementing or modifying the Rules cache-list generation path"
  - "Deciding fallback vs decoder-authoritative ownership for a resource family"
tags: [cache, decoder, manifest, coverage-assets, sound-rules, template-families, fallback]
related_components: [emukc_cache, emukc_model]
---

# Decoder-driven cache-list generation: manifest/category/coverage/rules integration

## Context

The cache list is generated from a mix of decoder-produced assets (decoded from
`main.js`) and legacy Rust fallback generators. The decoder assets are the
authoritative source for the resource families they cover: semantic rules,
category groups, sparse subsets, audio/UI coverage, sound rules, and
template-backed families. The generation path must let decoder coverage suppress
broad legacy fallback where the decoder is authoritative, while preserving
fallback for families the decoder has not yet proven. Every path must be
attributable as either rule-authored or fallback-authored so comparison
diagnostics correctly report residual gaps.

## Guidance

### Ship resource path generation

Generate cache-list paths for resolved ship entries using `SuffixUtils` and the
same path templates as `make_list/source/kcs2/resources/ship.rs`. When decoder
semantic rules exist for a ship target family, use those rules to decide which
target categories and selector scopes are valid. Only ship target families
WITHOUT an applicable decoder semantic rule MAY fall back to the legacy
damage-variant mapping table.

- A decoder-covered family marked damaged-only MUST generate ONLY the canonical
  damaged target categories the rule allows; it MUST NOT expand into undamaged
  or unrelated sibling variants via the legacy fallback table.
- A decoder-covered family that allows a base target plus a constrained variant
  set MUST emit the base target and ONLY the rule-allowed variants; selector
  scope (friendly, abyssal, graph-driven grouping) MUST stay constrained to the
  rule.
- A family with no decoder semantic rule MUST keep using the existing static
  variant mapping, and its output MUST remain identical to the current fallback
  implementation.

### Decoder category groups fill deterministic gaps

Use decoder category-group assets to generate deterministic ship and slot
categories visible in decoder output but absent from concrete manifest entries.

- A deterministic ship category present in decoder categories but absent from
  manifest entries (e.g. `power_up`) MUST be generated using the same templates
  as the existing bootstrap implementation.
- A deterministic slot category present in decoder categories but absent from
  manifest entries (e.g. `card_t`) MUST be generated the same way.

### Sparse subsets constrain non-universal categories

Use decoder sparse-subset assets to constrain categories whose membership is NOT
universal across all friendly ships or all slotitems.

- A sparse ship subset for a category such as `special`, `card_round`, or
  `reward_*` MUST limit output to the observed subset, not expand across all
  friendly ships.
- `sp_remodel` MUST apply its image-asset subset and its remodel-message subset
  INDEPENDENTLY, so `sp_remodel` output is not expanded to unrelated ships.

### Audio and UI coverage assets

Consume decoder audio and UI coverage assets to add non-ship/slot domains to the
cache list. When UI coverage assets enumerate concrete members for a family, the
Rules path MUST emit those members as decoder-authored output BEFORE invoking
legacy fallback; fallback stays responsible only for members the decoder bundle
did not prove.

- Audio coverage (sound effects, BGM, voice) MUST be included in the candidate
  cache list.
- UI coverage (map, furniture, useitem, area, world-select) MUST be included
  and attributable as rule-authored.
- When legacy UI fallback produces a path already proven by decoder UI coverage,
  the candidate list MUST preserve decoder-authored ownership; comparison
  diagnostics MUST NOT count that overlapping path as fallback-authored.

### Tolerance to partial coverage assets

Tolerate missing or partial decoder coverage assets WITHOUT aborting the
generation run.

- A missing/unreadable coverage asset for one domain MUST log a warning and
  continue with the remaining assets.
- A sparse category marked `partial` or `unresolved` MUST NOT be claimed as
  complete decoder coverage; fall back to existing bootstrap behavior for that
  category or skip decoder-only expansion for it.
- A UI asset proving only a concrete subset MUST emit the proven subset as
  rule-authored; residual members outside the proven set stay
  fallback-authored.

### Slot alias normalization

Apply decoder-authored slot normalization semantics BEFORE any universal
slotitem expansion for alias families such as `item_on2` and `item_up2`.

- A normalized alias family with a decoder rule MUST emit paths only for the
  normalized slot IDs the rule permits; it MUST NOT be treated as a universal
  slotitem category.
- An unresolved alias family MUST preserve existing fallback behavior and MUST
  NOT claim precise decoder coverage.

### Rules path consumes sibling decoder coverage assets

The `Rules` cache-list generation path, when built from a `cache_rules.json`
under a decoder output `resources/` directory, MUST load sibling decoder
coverage assets from that same directory, so audio/UI coverage and deterministic
category extensions are applied.

- Optional sibling coverage assets that are absent or unreadable MUST be treated
  as explicit fallback territory; generation MUST continue with the remaining
  bundle data.
- A malformed sibling asset (bad JSON / decode failure) MUST be treated as
  fallback territory, NOT abort the rules bundle load.

### Decoder-covered families suppress broad fallback

Treat decoder-covered families as authoritative. Invoke broad legacy expansion
ONLY for families that remain partial or unresolved in the decoder bundle.

- A covered family (e.g. `banner*`, `item_up2`, `item_on2`) MUST use the decoder
  rule as the authoritative selector; legacy universal expansion MUST NOT add
  sibling paths outside the rule's allowed set.
- An unresolved family MAY use fallback; paths produced via fallback MUST stay
  fallback-authored.

### Covered `kcs/sound` families from decoder sound rules

Generate covered `kcs/sound/*` families from decoder-authored sound rules
BEFORE consulting legacy Rust sound generators. Residual fallback MUST narrow as
decoder bucket coverage improves.

- A covered ship voice family MUST be generated from the decoder rule plus
  manifest data; the generator MUST NOT depend on Rust-only formula tables for
  that covered family.
- A covered non-ship sound bucket (e.g. `kc9997`, `kc9998`, `kc9999`) MUST be
  generated from the decoder bucket rule; the legacy Rust bucket generator fills
  ONLY the residual uncovered members.

### Suppress duplicate sound fallback for complete families

Avoid running broad legacy sound fallback generators for sound families the
decoder rule bundle marks complete. Preserve fallback for partial/unresolved
families.

- A complete decoder sound family MUST be generated from the rule; the matching
  legacy fallback generator MUST NOT insert the same family as
  fallback-authored.
- A partial decoder sound family MUST preserve legacy fallback for the unproven
  remainder; fallback paths MUST stay fallback-authored.
- When a decoder sound rule and a legacy fallback generator can produce the same
  path string, the Rules path MUST prevent complete families from being inserted
  again as fallback-authored; comparison output MUST NOT report fallback
  ownership for paths whose family is complete in decoder sound rules.

### Sound fallback stays explicit for unresolved families

Preserve existing Rust sound generators ONLY for sound families that remain
partial or unresolved. Fallback attribution MUST stay narrow enough to show
which members are still outside decoder authority.

- An unresolved sound family MAY use the legacy generator for the uncovered
  portion; those paths MUST be fallback-authored.
- A partially covered sound family MUST emit covered paths as rule-authored and
  report ONLY the uncovered remainder as fallback.

### Template-backed resource family expansion

Expand decoder-emitted template-backed families using declared runtime input
bindings BEFORE consulting legacy fallback.

- A complete template with all inputs available MUST be expanded into cache-list
  paths using the decoder-provided path template; expanded paths are
  rule-authored.
- A template with one or more inputs that cannot be loaded/validated MUST NOT be
  marked completely decoder-authored; fallback MAY be used and those paths MUST
  be fallback-authored.
- A partial template with a proven subset MUST emit that subset as
  rule-authored when its inputs are available; fallback stays responsible for
  residual members outside the proven subset.

### Suppress broad fallback for complete template families

Suppress broad legacy fallback expansion for template-backed families whose
descriptor and input bindings prove complete decoder-authoritative coverage.

- A complete template family MUST NOT be added again by matching legacy
  fallback generators; duplicate path strings MUST NOT inflate fallback
  ownership.
- A template proving only a subset MUST emit the subset as rule-authored;
  fallback stays available only for uncovered residuals and keeps
  fallback-authored attribution.
- When decoder template expansion emits map or gauge paths that a legacy
  generator can also produce, the candidate list MUST preserve rule-authored
  ownership; comparison diagnostics MUST NOT count those overlapping paths as
  fallback-authored residuals.

## Why This Matters

The decoder bundle is the only source that knows the true membership of
obfuscated/variable resource families (alias slots, sparse categories, sound
buckets, template-backed map/gauge families). Letting legacy universal fallback
run over decoder-covered families both wastes download work (requesting
non-existent resources) and corrupts the comparison diagnostics that track how
much of the cache list is decoder-authoritative vs fallback. The
attribution invariant (rule-authored vs fallback-authored, with no
double-counting) is what makes the "narrow the fallback over time" feedback loop
trustworthy.

## When to Apply

- When adding a new decoder asset type to the bundle — wire it through the Rules
  path with proper attribution and fallback tolerance.
- When a resource family gains decoder coverage — flip it from fallback to
  rule-authored and suppress the matching legacy generator.
- When comparison diagnostics show unexpected fallback ownership — check for a
  duplicate-path / double-counting violation of the suppression rules above.

## Examples

The Rules path loads `cache_rules.json` plus sibling coverage assets from the
same decoder `resources/` directory. A decoder-covered `banner*` family is
generated from its rule; legacy `banner` universal expansion is suppressed for
it. A complete `kcs/sound/kc9999` bucket rule generates its paths and the legacy
`kc9999` generator fills only the uncovered residual. A malformed sibling
coverage asset is logged and treated as fallback territory without aborting the
load.

## Related

- `manifest-damage-variants.md` — the legacy static variant table that decoder
  semantic rules override when they cover a family.
- `cache-list-dedup.md` — `(path, version)` dedup of the items these paths
  become.
- `cache-make-list-versioning.md` — version assignment applied to generated
  paths.
