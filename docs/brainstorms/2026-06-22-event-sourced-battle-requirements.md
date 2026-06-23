---
title: "Event-sourced battle simulation"
date: 2026-06-22
status: active
actors:
  - A1: Developer (battle system maintainer)
flows:
  - F1: Simulate day/night battle → produce event log → derive state → emit API packet
  - F2: Apply debug transforms (god_mode, one_hit_kill) as event stream filters
acceptance_examples:
  - AE1: SeededRng(seed=N) produces byte-identical API packet before and after refactor
  - AE2: god_mode transform zeroes all friendly damage events — HP unchanged in derived state
  - AE3: one_hit_kill transform sets every enemy hit to lethal — all enemies dead in derived state
  - AE4: Adding a new debug transform requires zero changes to simulation code
---

# Event-sourced battle simulation

## Summary

Rewrite the `emukc_battle` crate's core architecture from mutable-state simulation to event-sourced design. Simulation becomes a pure function that produces an event log. State (HP, sunk status) is derived by a separate reducer. Debug features (god_mode, one_hit_kill) are pure transforms on the event stream — no branching in simulation code.

## Problem

Current `apply_damage` mixes damage clamping, sinking protection, HP mutation, and debug overrides in one method with `if` branches. Adding any new damage modifier requires touching this method. The broader simulation has 109 `&mut` references spread across 8000 lines — debug features can't be added as wrappers, they must be embedded as branches.

## Actors

- **A1: Developer** — the maintainer who adds debug features, tests battle behavior, and debugs battle logic. The only user of this system.

## Flows

- **F1: Battle simulation** — `simulate(codex, context, rng) → Vec<BattleEvent>` (pure), then `reduce(events, initial_state) → BattleResult` (pure), then `to_packet(result) → BattlePacket` (pure). No side effects anywhere in the chain.
- **F2: Debug transform** — `god_mode(events: Vec<BattleEvent>) → Vec<BattleEvent>` — a pure transform on the event stream, applied between simulation and reduction. Adding one_hit_kill or any future debug feature is a new transform function, not a branch in simulation code.

## Success Criteria

- Every existing SeededRng-based test produces identical API packets before and after refactor (Covers AE1)
- god_mode transform zeroes friendly damage — derived HP unchanged (Covers AE2)
- one_hit_kill transform makes every enemy hit lethal — all enemies dead in derived state (Covers AE3)
- A new debug transform (e.g., "double damage") can be added without modifying any simulation function (Covers AE4)
- All 155 existing tests pass (updated to new assertion style)

## Scope Boundaries

### In scope

- Define `BattleEvent` enum and event log types
- Rewrite simulation phases as pure functions producing events
- Implement reducer that derives final state from events
- Implement debug transforms as event stream functions
- Rewrite all tests to match new architecture
- Maintain RNG determinism (identical draw sequence)

### Deferred for later

- CLI/UI toggle for debug flags
- Replay/debug tooling that visualizes event streams
- Immutable BattleState in gameplay crate (battle crate first)

### Outside this product's identity

- Multiplayer battle synchronization via event replication
- Client-side battle prediction from event logs

## Dependencies / Assumptions

- **RNG determinism is invariant**: SeededRng draw sequence must be identical. Golden transcript tests freeze current behavior.
- **API packet format is unchanged**: The client expects specific `api_hougeki`/`api_raigeki` structures. The packet layer adapts events to this format.
- **Performance is acceptable**: Event allocation per battle is negligible (battles are single-request scope, not hot paths).
