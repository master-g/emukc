---
title: "Material record read-modify-write concurrency protection"
date: 2026-06-22
category: best-practices
module: emukc_gameplay
problem_type: best_practice
component: database
severity: high
applies_when:
  - "Performing a read-modify-write on a player's material record"
  - "Running concurrent material mutations (replenish, quest rewards, expenditures)"
tags: [material, concurrency, read-modify-write, last-write-wins, database]
related_components: [emukc_db]
---

# Material record read-modify-write concurrency protection

## Context

A player's material record (fuel, ammo, steel, bauxite, and the consumable
buckets/torches/devmats/screws) is mutated by multiple independent gameplay
paths: timed replenish, quest rewards, craft expenditures, and store/airbase
operations. Each mutation is a read-modify-write: load the current values,
adjust, persist. Without concurrency protection, two overlapping writes race
and the loser's update is silently lost (last-write-wins overwrites the
winner's delta). Losing a replenish delta or a bucket spend corrupts the
player's economy with no error signal.

## Guidance

Every read-modify-write operation on a material record MUST have concurrency
protection, so that concurrent material writes do not lose updates via
last-write-wins.

Specifically, the values of `bucket`, `torch`, `devmat`, and `screw` MUST NOT
be accidentally overwritten by a replenish operation. A replenish that runs
concurrently with a spend/award on the same record must result in BOTH deltas
applied, not just one.

## Why This Matters

Material loss is silent and cumulative: there is no exception, no log, just a
record that drifts from the correct value. Because the consumable resources
(buckets, torches, devmats, screws) gate core gameplay loops (construction,
crafting, improvement), drift here directly breaks crafting and repair flows
and is extremely hard to audit after the fact. Concurrency protection (e.g.
row-level locking, optimistic version checks, or serialized transactions) makes
overlapping writes compose correctly instead of destroying each other.

## When to Apply

- When adding ANY new path that mutates a material record.
- When reviewing an existing `_impl` that does `load -> modify -> save` on
  materials — confirm it participates in the concurrency-protection scheme.
- When the material mutation participates in a larger transaction (it must
  still hold its protection within that transaction).

## Examples

A replenish task adds +3 buckets to a record currently at 10. Simultaneously a
craft spends 1 devmat on the same record. Without protection, the second write
clobbers the first and one delta is lost. With protection, both compose:
buckets become 13 and devmat decrements by 1, in a single consistent final
state.

## Related

- Gameplay `_impl` pattern (CLAUDE.md § Gameplay `_impl` Pattern) — material
  mutations typically run through `_impl` functions that join transactions.
