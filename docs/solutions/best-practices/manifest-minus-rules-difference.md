---
title: "The manifest-minus-rules difference is upstream art the client never loads"
date: 2026-09-21
category: best-practices
module: emukc_bootstrap
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - "Considering whether cache make-list misses resources the manifest strategy would catch"
  - "Deciding which ship resource categories apply to abyssal ships"
  - "Reading shipRules.targetSemantics in cache_rules.json"
tags: [manifest, make-list, cache-rules, target-semantics, abyssal, rejected-approach]
related_components: [main-decoder, emukc_cache]
---

# The manifest-minus-rules difference is upstream art the client never loads

## Context

`cache make-list` has two strategies. The default (Rules) strategy produced
73,031 entries on client 6.3.5.0; `--manifest` produced 94,541, and Rules is a
strict subset of it. A 2026-09-20 sample of the 21,510 manifest-only paths found
roughly 7% of them alive on the CDN, which read like a coverage gap in the
decoder's rules — 1,500-odd resources the emulator would never cache.

A full HEAD sweep of all 21,510 on 2026-09-21 (one keep-alive https connection
per CDN host, four minutes, zero errors) measured it properly:

- **786 exist** (3.65%), **20,724 are 404**, none was an empty 200.
- All 786 are the same thing: `kcs2/resources/ship/banner_dmg/` for abyssal ship
  ids, 786 of the 889 ids in 1501..2397. The missing 103 are two contiguous
  blocks, 1846-1920 and 2063-2090.
- Nothing else in the difference exists. Not one abyssal `album_status`, `card`,
  `remodel`, `supply_character`, no friendly `banner3`, no `slot/item_on2`.

## Guidance

**Do not add abyssal `banner_dmg` to the cache list.** The files exist upstream,
but the client never requests them, and `cache_rules.json` proves it. Its
`shipRules.targetSemantics` table (`coverageMode: observed-complete`, decoded
from main.js) maps a raw target type plus a selector scope plus a damaged state
to the concrete categories the client loads:

```
banner  scope=default-friendly  dmg=true  -> ["banner_dmg"]
banner  scope=default-abyssal   dmg=true  -> ["banner"]
banner3 scope=default-abyssal   dmg=true  -> ["banner3"]
```

A damaged abyssal ship keeps its intact banner. Only friendly ships switch to
the `_dmg` artwork. Caching those 786 files would download art no client ever
asks for.

More generally: **the CDN is not the authority on what to cache, the client is.**
"The file exists upstream" answers a different question from "the client loads
it". When the two disagree, `targetSemantics` wins.

## Why This Matters

The obvious fix looks right and is wrong. Adding `"banner_dmg"` to
`defaultAbyssal` in `main-decoder/src/resource-categories.ts` is a one-line
change that survives `bun run check`, `bun test`, and a full decode+sync — and
it changes the generated list by exactly **zero entries**, because
`ship_semantic_targets_for_id` consults `targetSemantics` before the generation
groups are ever reached. The generation groups are only a fallback for targets
the semantic table does not cover. A reviewer who checked only that the code
compiled would have merged a no-op plus a 103-id hole table for resources that
are never generated.

## When to Apply

- Before treating a manifest-vs-rules difference as a coverage gap: probe it,
  then check `targetSemantics` for the target before changing any group list.
- When adding a resource category for abyssal ships: the scope-aware cases in
  `targetSemantics` are the client-derived answer;
  `shipGenerationGroups.defaultAbyssal` in `resource-categories.ts` is a
  hand-authored candidate list and only a fallback.

## Related

- `docs/solutions/best-practices/manifest-damage-variants.md` — the base-to-variant
  mapping this table sits above.
- `docs/solutions/best-practices/cache-manifest-integration.md` — why the Rules
  strategy is the default and the manifest fallback is not expanded blindly.
- `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` — the audit that raised
  the 7% estimate this measurement closes.
