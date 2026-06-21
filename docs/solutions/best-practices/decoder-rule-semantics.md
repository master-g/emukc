---
title: "Decoder ship and slot semantic rules contract"
date: 2026-06-22
category: best-practices
module: main-decoder
problem_type: tooling_decision
component: tooling
severity: high
applies_when:
  - "Authoring decoder-derived ship or slot semantic rules for cache-list generation"
  - "Deciding whether a target family suppresses legacy variant fallback"
tags: [decoder, semantic-rules, ship, slot, damaged-only, normalization]
related_components: [emukc_bootstrap]
---

# Decoder ship and slot semantic rules contract

## Context

Decoder-derived ship and slot semantic rules narrow cache-list generation
beyond raw manifest extraction. They encode canonical target behavior
(damaged-only, variant-expandable, group-scoped) and slot normalization
semantics, so downstream generation does not infer disallowed siblings from
raw target names. This contract documents what the rules must encode and the
completeness controls that govern fallback suppression.

## Guidance

**Ship semantic rules — canonical variant scope.**

- Represent decoder-derived ship rule semantics as canonical target behavior:
  whether a target is base, damaged-only, or variant-expandable, plus the
  ship selector scope (friendly, abyssal, graph-driven, or sparse-subset)
  that may generate that target.
- When decoded `main.js` usage shows a ship target family such as `banner_g`,
  `banner2_g`, or `banner3_g` is only valid in damaged form, encode that
  family as damaged-only; downstream generation must not infer undamaged
  sibling targets from the raw target name alone.
- When decoded runtime usage differs between friendly ships, abyssal ships,
  or graph-driven ship groups, preserve those selector boundaries explicitly;
  downstream generation must emit the allowed group without expanding into
  disallowed groups.

**Ship semantic completeness — fallback suppression control.**

- Distinguish complete decoder ship target-family semantics from partial
  observations before downstream generation treats those semantics as
  authoritative.
- When evidence contains `banner_g`/`banner2_g`/`banner3_g` signals but does
  not prove complete semantic coverage for the banner family, mark that
  family partial or unresolved; downstream generation must preserve legacy
  variant fallback for the unproven remainder.
- When evidence proves the complete semantic scope for a ship target family,
  identify that family as complete with the allowed target semantics and
  selector scope; downstream generation must treat those semantics as
  authoritative.
- Hardcoded semantic case definitions in the decoder implementation must not
  be emitted as observed-complete merely because any family member was
  observed. Emitted completeness must reflect decoder evidence for the
  family, not the presence of Rust- or TypeScript-authored fallback constants.

**Slot semantic rules — normalization-scoped alias families.**

- Represent decoder-derived slot rule semantics for normalization-driven
  target families so alternate slot targets are modeled as constrained
  aliases of observed runtime selectors rather than universal slotitem
  categories.
- When decoded usage shows a target family such as `item_up2` or `item_on2`
  is produced from a specific runtime slot selector or normalization rule,
  preserve that selector and normalization behavior explicitly; downstream
  generation must not expand that family across all slotitems solely because
  the raw target exists.
- When the decoder cannot fully derive the normalization or selector scope,
  mark the family partial or unresolved; downstream generation must treat it
  as a fallback case instead of claiming precise decoder semantics.

## Why This Matters

Without these rules, downstream generation infers siblings from raw target
names (e.g. assuming a `banner_g` implies a `banner`), producing paths the
client never loads. The completeness control is equally load-bearing: emitting
a family as complete based on a hardcoded case definition rather than decoder
evidence suppresses fallback prematurely and hides gaps.

## When to Apply

- When the decoder newly observes a ship or slot target family.
- When reviewing whether a family is safe to remove from legacy fallback.

## Examples

- `banner_g` is observed only in damaged contexts: the rule encodes
  damaged-only, and generation does not emit an undamaged `banner` from the
  name.
- `item_up2` is produced via runtime normalization of a specific selector: the
  rule encodes the normalization, and generation does not treat `item_up2` as
  a universal slotitem category.

## Related

- `docs/solutions/best-practices/decoder-coverage-assets.md`
- `docs/solutions/best-practices/decoder-sound-rules.md`
- `docs/solutions/best-practices/cache-manifest-integration.md`
