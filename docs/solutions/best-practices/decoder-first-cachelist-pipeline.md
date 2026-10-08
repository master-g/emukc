---
title: "Decoder-first cache-list pipeline contract"
date: 2026-06-22
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: high
applies_when:
  - "Generating cache lists from a decoder rule bundle instead of legacy fallback"
  - "Evaluating migration readiness of the decoder-first pipeline"
tags: [decoder, cachelist, pipeline, migration, authority]
related_components: [emukc_cache]
---

# Decoder-first cache-list pipeline contract

## Context

The decoder-first cache-list pipeline generates a candidate cache list from
an explicit decoder rule bundle (`cache_rules.json` plus sibling coverage
assets), classifies output by authority stage (rule-authored vs
fallback-authored), and exposes migration-readiness diagnostics. This
contract documents bundle loading, authority accounting, the sideband
diagnostics shape, and the path-generation invariants the pipeline must hold.

## Guidance

**Bundle loading.**

- Support generation from an explicit decoder bundle rooted at
  `cache_rules.json`, together with sibling coverage assets from the same
  decoder output tree and the runtime manifest/version inputs already
  available to bootstrap.
- When a caller provides a `cache_rules.json` path from a decoder output
  `resources/` directory, load that rules asset as the primary input and
  resolve sibling coverage assets from that same tree before consulting
  repo-tracked bootstrap assets.
- When invoked without an explicit path, load the repo-tracked decoder bundle
  from `crates/emukc_bootstrap/assets/` using the same generation flow.

**Authority separation.**

- Classify generated output by authority stage: paths produced directly from
  decoder rules are rule-authored; paths produced only by legacy fallback are
  fallback-authored.
- For a decoder-covered ship/slot/audio/UI family, record generated paths as
  rule-authored; broad legacy fallback must not re-expand that family outside
  the decoder rule's allowed scope.
- For an unresolved family, legacy fallback may preserve generation
  continuity, but the resulting paths must be recorded as fallback-authored
  with an attributable residual key or family label.

**Shipgraph exclusion invariants.**

- When resolving ship IDs for a `friend_graph` target
  (`character_full`, `character_up`, etc.), exclude shipgraph entries where
  `api_sortno == Some(0)`; those entries must not produce `character_full`,
  `character_full_dmg`, `character_up`, or `character_up_dmg` paths.
- When a shipgraph entry has `api_id >= 5000` but does not exist in
  `api_mst_ship`, exclude it via the `event_ship_holes` mechanism; no
  character paths shall be generated for excluded event ships.

**Sideband diagnostics (payload format unchanged).**

- Expose unresolved rule keys, fallback-dependent families, and authority
  totals as sideband diagnostics. The serialized cache-list output must remain
  limited to the existing `_id`, `path`, and optional `version` fields.
- When generation finishes with residual fallback usage, diagnostics must
  include unresolved rule keys or grouped residual families.
- When generation finishes with no decoder-authority blockers, diagnostics
  must indicate that; the payload format still stays unchanged.

**Template-backed authority accounting.**

- Paths expanded from complete decoder template-backed family descriptors
  (template metadata plus validated runtime inputs all satisfied) are
  rule-authored; the same family must not be reported as fallback-dependent
  just because legacy Rust generators contain an equivalent path formula.
- For a template-backed family whose descriptor, provenance, completeness
  mode, or input binding is partial or unresolved, keep it in fallback
  territory for unproven paths; diagnostics must include an attributable
  residual key or family label.
- When fallback-authored output remains for a template-backed family,
  diagnostics must identify whether the residual came from missing descriptor
  evidence, partial coverage mode, unavailable runtime input, or uncovered
  member residuals — so migration-readiness checks can use that reason without
  inspecting individual path strings.
- Template-backed ownership diagnostics (family labels, completeness state,
  residual counts) are sideband data; the serialized cache-list items remain
  unchanged. When a template family emits both rule-authored and
  fallback-authored paths, report both counts under the same stable family
  label.

**Explicit path generation.**

- Reject directory-like paths that lack a trailing slash: a path like
  `resources/voice` or `resources/friendly_panel/e` must be recognized as a
  directory reference and excluded from the cache list.
- Preserve file paths with extensions (e.g. `resources/stype/etext/sp001.png`)
  as normal.

**Template area path expansion.**

- Template-backed area families (`airunit`, `airunit_extend_confirm`) must
  generate paths only for map areas known to have the corresponding resources.
  When decoder UI assets provide observed area IDs, expand only for those;
  areas without observed evidence (e.g. areas 001-005 for airunit) must not
  produce paths.
- When decoder UI assets are absent, fall back to the hardcoded area ID list
  from the unversioned fallback generator.

**Template gauge path expansion.**

- Template-backed gauge families must generate JSON paths only for map IDs
  present in the known gauge map set (regular EO maps and event maps).
  Regular non-EO maps (e.g. 1-1, 2-1) must not produce gauge JSON paths.

## Why This Matters

The authority separation is what makes migration-readiness auditable. If
fallback output is silently mixed with rule-authored output, the comparison
example cannot tell whether a shrinking residual came from real decoder
progress or from a regression that re-expanded legacy fallback. The
payload-format invariant ensures the pipeline can be adopted without
breaking downstream cache consumers.

## When to Apply

- When implementing or modifying the decoder-first generation path.
- When adding a new template-backed family to the pipeline.

## Examples

- A decoder-covered `banner*` family emits rule-authored paths; legacy
  universal expansion does not add sibling paths outside the rule's allowed
  set. The residual section is empty.
- A shipgraph entry with `api_sortno == 0` is excluded from
  `character_full` generation even though it appears in the shipgraph table.

## Related

- `docs/solutions/best-practices/decoder-cachelist-comparison.md`
- `docs/solutions/best-practices/decoder-coverage-assets.md`
- `docs/solutions/best-practices/cache-manifest-integration.md`

## Checking the list from the client's side (2026-10-08)

A way of building an address that the decoder does not recognise is left out of the list
without a trace: `cache_rules.json` reported no unresolved rules while whole families were
missing. `make cache-list-oracle` (`main-decoder/src/cache-list-oracle.ts`) looks from the
other side, in five checks:

1. Every directory the client names in a string literal has something listed under it.
   Addresses built by hand (`"resources/ship/".concat(…)`) are the ones rule extraction
   misses, and they all carry such a literal. Rightly empty ones are in `KNOWN_EMPTY`.
2. Every resource type a ship or equipment loader is called with (289 call sites, 35
   types, from `resource_manifest.json`) has something listed under it.
3. The suffix of every listed ship and equipment address is the one the client's
   `SuffixUtil` computes.
4. The client's own `ShipLoader.getPath` runs in Bun, answering from `start2` what it
   would ask the master data models. For every ship that has a type listed, every damaged
   state a call site can ask of that type, and the broken look where a call site passes
   it, is asked of the loader; the address it answers must be listed, at the version it
   answers (36,968 addresses on 6.3.5.0).
5. Families whose ids are kept by hand (area banners, map files, use item cards) are swept
   against the ids of the master data.

An address the origin does not have is a hole, not a difference: the generator's own hole
tables are read from the manifest's path rules, and the rest are recorded with their reason
in `main-decoder/cache-list-known-holes.json`. To learn whether a reported address exists,
write the reported lines to a file and run `cache populate --src` on it; it downloads what
the origin has and names the rest.

It says what it does not judge: which areas have air bases and which event files exist
come from the server's own map data, and `sp_remodel/animation_key` uses a file name it
does not parse. `ShipLoader.hasai` (`_d` after the id for a boss the battle marks damaged)
and `enemy_overwrite_force` come from battle data of events and are not asked.
`cache_rules.json`'s "unresolved 0" still only counts the rules the decoder attempted.

It runs after the drift report of `make update`, without blocking, against the list as it
is; regenerate the list before reading it.

What it found on 6.3.5.0, all confirmed on the origin:

| Missing from the list | Why the client asks | Files |
|---|---|---|
| `ship/full_animation`, `full_animation_dmg` | an owned ship with `api_sp_flag` 1 is drawn animated in port | 6 |
| `area/airunit_extend_confirm` | the dialog for buying an air base, regular areas | 4 |
| abyssal `banner_dmg`, `banner3_dmg` and `_b` files | on an abyssal graph `api_sp_flag` 1 is a second, broken look | 54 |
| `ship/special_dmg` | two cut-ins load `special` with the attacker's damaged state | 9 |
| `useitem/card` 104, 105; `card_` 102, 104, 105 | the hand lists had not followed new items | 5 |

The first two came from check 1, the abyssal files from reading call sites (check 4 now
covers them), the last two from checks 4 and 5.
