---
title: "Routing rules are converted from the compass simulator source"
date: 2026-10-06
category: architecture-patterns
module: emukc_bootstrap
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Changing how routing rules reach the map catalog"
  - "Upgrading the pinned compass simulator commit"
  - "Adding a routing predicate or a counter to RouteCounter"
  - "A routing rule looks wrong and you need to know where it came from"
---

# Routing rules are converted from the compass simulator source

## What

`crates/emukc_bootstrap/assets/map_route_rules.json` is generated, end to end and without an
LLM, from the per-map branch functions of the compass simulator
(`X-20A/X-20A.github.io`, branch `compass_dev`, MIT). The pipeline:

```
make route-rules-update
  cargo run -- route-rules sync        # pinned commit -> .data/temp/x20a_compass/<sha>/
  cd main-decoder && bun run route-rules   # TypeScript -> neutral JSON
  cargo run -- route-rules normalize   # neutral JSON -> the asset
  cargo run -- battle drift-check      # report what changed
make route-oracle                      # compare the result with the source's own code
```

The catalog build then pins the label-space rules onto cells (`apply_route_rules`),
replacing the routing rules of the wikiwiki catalog, which is kept for enemy fleets only.

## Decisions worth knowing

- **The default branch is not the source.** `main` of that repository holds minified build
  output and no license. The readable, MIT-licensed source is on `compass_dev`.
- **One pinned commit.** `COMPASS_SOURCE_COMMIT` in `compass_source.rs` is the only place the
  commit is named; the decoder reads it from there. The unpack step refuses a tree whose
  `LICENSE` no longer begins `MIT License`.
- **Whitelist, not best effort.** The parser accepts the constructs the 37 regular maps use
  and fails with `file:line` on anything else; normalize fails on an unknown ship type,
  operator or phase; catalog assembly fails on a rule the topology cannot carry. Nothing
  becomes an `Unknown` predicate. The previous pipeline's failures were all quiet ones.
- **The source's order is the router's priority.** The source runs `if`s in order and the
  first match wins. The router already runs the lowest-priority matching group, with
  priority being the rule's position, so no runtime change was needed. Every target of one
  weighted `return` must carry the *same* predicate: the router groups rules by predicate,
  and targets with different predicates would compete instead of sharing one roll.
- **`CountSum` instead of one predicate per shape.** The source compares arithmetic over
  counts (`BBs - SBB_count >= 2`, `CA + CL + Ds === ships_length`). A weighted sum of
  `RouteCounter`s maps onto that one to one; recognising each shape as `ShipTypeCount`,
  `OnlyShipTypes` and so on would have meant pattern-matching the source.
- **Start points.** The source's `case null` chooses between start `1` and `2` by fleet. Those
  rules are `start_rules` on the variant, read by `route_start_cell`. Both start cells are
  labelled `Start` in the catalog; `1` is the one with the lower cell number.
- **Phases.** 7-3's two phases are its two variants. 5-6's three phases go to its four
  phase variants, and a map with one set of rules but several phases (7-2, 7-5) gets it on
  each. An earlier phase drops the rules it has no cells for. See `map-gauge-phases.md`.

## Upgrading the source

1. Change `COMPASS_SOURCE_COMMIT`.
2. `make route-rules-update`. A parser failure names the file and line of a construct it
   does not know; extend the parser and its tests rather than working around it.
3. `cargo run -- bootstrap --codex-only`, then `make route-oracle`. It must report no
   difference between the source and the router.
4. Read the asset diff, `make drift-accept`, and say in the commit what the source changed.

## Limits

- The source encodes the Japanese wiki's branching rules by hand. The oracle shows the
  conversion is faithful to the source; it cannot show the source is right about the game.
- Probabilities the wiki gives as 「n 寄り」 without a number are the source author's
  estimates and are indistinguishable from measured ones in the code.
- The oracle reached 1,016 of 1,037 rules at commit `4f32c40e`. The rest need fleets its
  generator does not build; a few need more than six ships.
