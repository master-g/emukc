---
title: "Decoder coverage assets bundle contract"
date: 2026-06-22
category: best-practices
module: main-decoder
problem_type: tooling_decision
component: tooling
severity: high
applies_when:
  - "Authoring or extending decoder coverage extraction for cache-list generation"
  - "Deciding whether a cache-list family is decoder-authoritative or fallback territory"
tags: [decoder, coverage-assets, sparse-subsets, audio, ui, template-backed]
related_components: [emukc_bootstrap]
---

# Decoder coverage assets bundle contract

## Context

The decoder emits a structured coverage asset bundle (sparse subsets, audio,
UI, semantic rules, sound rules, template-backed families) that downstream
bootstrap cache-list generation consumes to narrow legacy fallback. This
contract documents what each asset must contain and the honesty rules that
prevent the decoder from claiming coverage it cannot prove from decoded
`main.js`.

## Guidance

**Sparse ship and slot subsets.**

- Emit a structured coverage asset bundle for sparse ship/slot categories
  whose membership is directly observable in decoded `main.js`. Each subset
  must include the category key, resource domain, observed IDs, provenance,
  and an explicit coverage mode distinguishing complete from partial or
  unresolved observation.
- Categories with decoder-observable literal evidence must not remain
  unresolved merely because the extractor fails to connect that evidence to
  the emitted subset.
- Runtime-driven subsets that cannot be enumerated from decoded `main.js`
  must be marked `partial` or `unresolved`. Never synthesize completeness by
  copying Rust constants or CDN-derived IDs.

**Audio coverage asset.**

- Record directly observable cache-list audio domains: sound-effect IDs,
  categorized BGM IDs, and voice-related ranges or explicit file stems.
- When decoded modules expose numeric sound-effect IDs or categorized BGM IDs
  through explicit paths, call arguments, inline tables, or other observable
  structures, store them under stable domain keys. Do not leave categorized
  `port`, `battle`, or `fanfare` BGM coverage empty when observable evidence
  exists.
- For voice domains exposed as ranges or file stems (titlecall, tutorial
  voice), preserve the range/stem form rather than flattening into invented
  IDs.

**UI coverage asset.**

- Record explicit file groups for non-ship/slot UI domains: map, furniture,
  useitem, area, and world-select resources. For migration-critical families,
  emit concrete members instead of leaving groups empty; preserve
  partial/unresolved modes when the script does not prove full membership.
- Represent `useitem/card` and `useitem/card_` as separate groups.
- Preserve `area/sally`, `area/airunit`, `area/airunit_extend_confirm`
  members under stable area domain keys; keep unresolved groups partial.
- Preserve `worldselect` concrete files under a stable world-select key.

**Output and sync.**

- Write coverage assets into the normal decoder output tree on every run.
- With the asset sync workflow enabled, also sync them into
  `crates/emukc_bootstrap/assets/`. Without the sync flag, do not modify
  repo-tracked bootstrap assets.

**Ship semantic scope in `cache_rules.json`.**

- For target families whose generation behavior cannot be represented by raw
  manifest entries alone, encode the canonical semantic behavior: base,
  damaged-only, or group-scoped, with selector scope (friendly, abyssal,
  graph-driven, or sparse-subset).
- When full semantic scope cannot be derived from decoded `main.js`, mark the
  rule partial or unresolved. Never synthesize complete scope by copying
  Rust-authored fallback tables.

**Slot normalization semantics in `cache_rules.json`.**

- For normalization-driven or alias slot families (`item_up2`, `item_on2`),
  encode the normalization and selector constraints needed to reproduce the
  family precisely without treating it as a universal slotitem category.
- Mark families partial or unresolved when normalization cannot be fully
  derived; never claim complete precision.

**Decoder-authored artifact honesty.**

- Derive ship and slot semantic rule outputs from decoded `main.js` evidence
  with provenance; never use Rust-authored path-rule constants as the source
  of truth for semantic meaning.
- Rules must be stable when regenerated from the same decoded artifact set.
- When evidence is insufficient, leave the rule partial or unresolved; do not
  backfill by parsing Rust fallback constants.

**Audio vs algorithmic sound-rule distinction.**

- Keep explicit audio asset extraction (`se`, `bgm`, titlecall, tutorial
  voice, explicit voice files) distinct from algorithmic `kcs/sound/*` rule
  extraction. A direct explicit asset path must not require an algorithmic
  rule, and a rule-driven family must not be flattened into explicit audio
  lists.

**Sound-rule metadata.**

- For covered `kcs/sound/*` families, preserve the bucket identity, reachable
  voice IDs, and any semantic grouping needed for downstream generation.
  Metadata must be stable enough to regenerate the same bundle from the same
  decoded script version.
- Mark families partial or unresolved when evidence is incomplete; never
  silently claim complete coverage.

**Template-backed resource families.**

- For resource families whose path shape is a deterministic template but
  whose member set depends on runtime inputs, emit structured metadata:
  stable family key, resource domain, path template, required input bindings,
  coverage mode, decoded-module provenance, and completeness information.
- When the template is proven but membership is runtime-bound, declare the
  required input binding explicitly; do not synthesize the missing member set
  from Rust constants, CDN-derived lists, or generated cache-list output.
- For migration-critical partial template families (`map.base`, `gauge.map`,
  `bgm.category`, `sound.kc9998`), preserve the observed path-template
  evidence and expose the missing descriptor/family-boundary/runtime-input
  reason.

**Template path authority vs member completeness.**

- Distinguish path-template authority from member-set completeness: a family
  may be decoder-authoritative for path shape while remaining partial or
  unresolved for membership until its required runtime inputs are available.
  Downstream generation must be able to decide ownership from the descriptor
  and input availability rather than treating the family as opaque fallback.

## Why This Matters

These assets are the input to decoder-first cache-list generation. If the
decoder over-claims coverage (synthesizing IDs from Rust constants or CDN
scrapes), the comparison example will report false migration-readiness. If it
under-claims (leaving observable subsets unresolved), fallback never narrows.
The honesty rules are the load-bearing invariant.

## When to Apply

- When extending the decoder to cover a new cache-list domain.
- When reviewing whether a family can be removed from legacy fallback.

## Examples

- A decoded module exposes `resources.getShip(id, true, "banner_g")` only for
  damaged ships: the rule must encode `banner_g` as damaged-only, and
  downstream generation must not infer an undamaged `banner` sibling from the
  name.
- Decoded modules expose `getTexture("COMMON_MISC", 1, 2, 5)`: the audio/UI
  asset stores those IDs under a stable key rather than leaving the family
  for universal expansion.

## Related

- `docs/solutions/best-practices/decoder-first-cachelist-pipeline.md`
- `docs/solutions/best-practices/decoder-rule-semantics.md`
- `docs/solutions/best-practices/decoder-sound-rules.md`
- `docs/solutions/best-practices/cache-manifest-integration.md`
