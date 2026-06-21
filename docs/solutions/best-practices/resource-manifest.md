---
title: "Resource manifest extractor contract"
date: 2026-06-22
category: best-practices
module: main-decoder
problem_type: tooling_decision
component: tooling
severity: high
applies_when:
  - "Extracting resource loading patterns from decoded main.js into resource_manifest.json"
  - "Integrating the extractor with the decode pipeline via --sync-resource-manifest"
tags: [resource-manifest, extractor, decoder, getship, getslotitem, gettexture]
related_components: [emukc_bootstrap, emukc_cache]
---

# Resource manifest extractor contract

## Context

The resource manifest extractor scans decoded `main.js` modules for resource
loading patterns (`getShip`, `getSlotitem`, `getTexture`, explicit paths) and
emits a JSON manifest consumed by bootstrap cache-list generation. This
contract documents the scan scope, pattern matching, deduplication, output
format, and the HTTP 404 fast-fail rule for remote fetches.

## Guidance

**Scan scope — all modules.**

- Traverse all decoded modules (not just battle-tagged modules) to discover
  resource loading patterns.
- A non-battle module calling `resources.getShip(id, damaged, type)` must
  produce an entry; a battle module calling the same must also produce an
  entry (no battle-only filtering).

**Ship resource pattern extraction.**

- Match `resources.getShip(id, damaged, type)` and `ShipLoader.add(id,
  damaged, type)` call expressions across all modules, extracting id source
  expression, damaged source expression, and target type string literal.
- `resources.getShip(vo.ship.api_id, false, "full")` produces an entry with
  kind `ship`, source `resources.getShip`, shipMstIdSource `vo.ship.api_id`,
  damagedSource `false`, targetType `full`.
- `ShipLoader.add(shipId, isDamaged, "banner")` produces an entry with
  source `ShipLoader.add`, shipMstIdSource `shipId`, damagedSource
  `isDamaged`, targetType `banner`.
- `resources.getShip(...args)` (spread arguments) is skipped; no entry.

**Slotitem resource pattern extraction.**

- Match `resources.getSlotitem(id, type)` and `SlotLoader.add(id, type)`
  across all modules, extracting id source expression and target type string
  literal.
- `resources.getSlotitem(eq.api_id, "card")` produces kind `slotitem`,
  source `resources.getSlotitem`, slotMstIdSources `["eq.api_id"]`,
  targetType `card`.
- `SlotLoader.add(itemId, "item_on")` produces source `SlotLoader.add`,
  slotMstIdSources `["itemId"]`, targetType `item_on`.

**Texture provider pattern extraction.**

- Match `getTexture(provider, ...ids)` across all modules, extracting
  provider name and numeric texture ID arguments.
- `textures.getTexture("COMMON_MISC", 1, 2, 5)` produces kind
  `texture-provider`, provider `COMMON_MISC`, textureIds `[1, 2, 5]`.
- Two calls `getTexture("FOO", 1)` and `getTexture("FOO", 2)` merge into one
  entry with textureIds `[1, 2]`.

**Explicit path extraction.**

- Scan all module source code for `kcs2/resources/[A-Za-z0-9_./-]+` patterns
  and collect unique explicit resource paths.
- A module containing `kcs2/resources/battle/banner/001_abc.png` includes that
  path in an explicit-path entry.
- Duplicate paths across modules appear once.

**Output format.**

- Output a JSON file containing `version` (integer 1), `generatedAt` (ISO
  8601 string), and `entries` (array of resource entries).
- Each entry includes `kind`, `moduleId`, and `moduleName` provenance. An
  entry from module `abc123` named `BattleRenderer` contains moduleId
  `abc123` and moduleName `BattleRenderer`.

**CLI flag integration.**

- The decode pipeline accepts a `--sync-resource-manifest` flag that triggers
  extraction and writes output to
  `crates/emukc_bootstrap/assets/resource_manifest.json`.
- Without the flag, the extractor does not run.

**Deduplication.**

- Deduplicate before output: ship entries by `(targetType, source)`;
  slotitem entries by `(targetType, source)`; texture-provider entries by
  `(provider)`; explicit-path entries by individual path.
- Two modules both calling `resources.getShip(id, false, "full")` yield one
  ship entry with both moduleIds in its provenance.

**No modification to existing extractors.**

- The resource manifest extractor must not modify the behavior, output, or
  interface of the existing `battle-knowledge.ts` extractor.
- With both `--sync-battle-assets` and `--sync-resource-manifest` provided,
  battle knowledge assets are identical to running with
  `--sync-battle-assets` alone.

**fetch_from_remote HTTP 404 fast-fail.**

- When `fetch_from_remote` in `emukc_cache::kache` receives an HTTP 404 from
  any CDN node, return `FailedOnAllCdn` immediately without attempting
  remaining CDN nodes.
- Non-404 errors (connection failures, timeouts, 5xx) continue to fall
  through to the next CDN node as before.
- First CDN returns 404: no other nodes tried, `FailedOnAllCdn` returned.
- First CDN times out, second returns 200: resource downloaded from the
  second (existing fallback preserved).

## Why This Matters

The extractor is the upstream of all pathRules and cache-list generation.
Scanning only battle modules would silently drop resource families loaded
elsewhere (UI, port, practice). The 404 fast-fail rule prevents the cache
from burning through all CDN nodes for a resource the server has confirmed
does not exist.

## When to Apply

- When extending the extractor to recognize a new resource loading pattern.
- When debugging a missing resource family in the generated cache list.

## Examples

- A non-battle UI module calls `resources.getShip(id, false, "full")`: the
  entry is included despite the module not being battle-tagged.
- `fetch_from_remote` gets a 404 from the first CDN: it returns
  `FailedOnAllCdn` immediately rather than trying the remaining nodes.

## Related

- `docs/solutions/best-practices/pathrules-loading.md`
- `docs/solutions/best-practices/web-asset-bootstrap.md`
- `docs/solutions/logic-errors/cache-list-character-holes-exclusion-2026-06-15.md`
