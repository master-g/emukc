---
title: "RNG facade: single-point GameRng backend with thread-local and seeded modes"
date: 2026-06-22
category: architecture-patterns
module: emukc_crypto
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Generating random values anywhere in the workspace"
  - "Implementing deterministic battle replay"
  - "Swapping the PRNG backend"
tags: [rng, crypto, gamerng, deterministic, thread-local, seeded, replay]
related_components: [emukc_battle, emukc_gameplay]
---

# RNG facade: single-point GameRng backend with thread-local and seeded modes

## Context

All randomness in EmuKC flows through one facade: `emukc_crypto::rng`. It
provides two modes — thread-local free functions for production, and seeded
`GameRng` instances for deterministic battle replay. Encapsulating the backend
in one module means the PRNG algorithm can be swapped without touching any
caller.

## Guidance

The following invariants hold for the RNG facade:

### Seeded instance

- **Deterministic from a `u64` seed.** `GameRng::seeded(seed)` SHALL produce
  identical output sequences for the same seed across invocations.
- **Exclusive integer range.** `game_rng.i64(min..max)` SHALL return a value
  in `[min, max)` deterministically based on the seed.

### Inclusive range support

- **Inclusive bounds.** The facade SHALL accept `RangeInclusive` bounds
  (`min..=max`), abstracting the backend's exclusive-only limitation.
- **Thread-local inclusive.** `rng::i64_inclusive(3..=6)` SHALL return a value
  in `[3, 6]` (inclusive on both ends).
- **Seeded inclusive.** `game_rng.i64_inclusive(0..=3)` SHALL return a value
  in `[0, 3]` deterministically.
- **Type-max inclusive without overflow.** `rng::i64_inclusive(i64::MAX..=i64::MAX)`
  SHALL return `i64::MAX` without overflow (saturating conversion).

### Float range

- **`f64_range`.** The facade SHALL provide `f64_range(min, max)` returning a
  random `f64` in `[min, max)`.
- **Thread-local float.** `rng::f64_range(0.0, 100.0)` returns a value in
  `[0.0, 100.0)`.
- **Seeded float.** `game_rng.f64_range(0.3, 1.5)` returns a value in
  `[0.3, 1.5)` deterministically.

### Thread-local free functions

- **Free functions.** The facade SHALL provide free functions (`usize`, `i64`,
  `u32`, `f64`) returning random values from thread-local state with no
  explicit RNG instance.
- **Thread-local integer range.** `rng::usize(0..len)` returns a `usize` in
  `[0, len)`.
- **Thread-local unit float.** `rng::f64()` returns an `f64` in `[0.0, 1.0)`.

### Collection helpers

- **`shuffle`.** `rng::shuffle(&mut slice)` SHALL randomly permute the slice
  in place.
- **`choose` from non-empty slice.** `rng::choose(slice)` SHALL return
  `Some(&element)` with uniformly random selection.
- **`choose` from empty slice.** `rng::choose(&[])` SHALL return `None`.
- **`choose_iter` from non-empty.** `rng::choose_iter(iter)` SHALL return
  `Some(&element)` with uniformly random selection.
- **`choose_iter` from empty.** `rng::choose_iter(std::iter::empty())` SHALL
  return `None`.

### Backend encapsulation

- **Single-point backend.** The RNG backend SHALL be encapsulated in
  `emukc_crypto::rng` so that swapping the PRNG algorithm (e.g., fastrand →
  biski64) requires modifying only that module; no other crate or file SHALL
  require modification.

### Battle deterministic replay

- **Seeded battle determinism.** `BattleRandom` in
  `crates/emukc_gameplay/src/game/battle/core.rs` SHALL use `GameRng` for
  seeded mode: a battle run with `rng_seed: Some(N)` SHALL always produce the
  same outcome for the same seed.
- **Unseeded fallback.** A battle run with `rng_seed: None` SHALL use
  thread-local RNG free functions.

## Why This Matters

Deterministic replay is the foundation of the battle test harness: the golden
transcript and `seed-search` CLI depend on a seeded run reproducing exactly.
Leaking randomness to any other source (e.g., a direct `rand` call) silently
breaks replay. The single-point backend also makes an algorithm swap a
one-file change instead of a workspace-wide audit.

## When to Apply

- When writing any code that needs randomness — go through `emukc_crypto::rng`,
  never call a backend PRNG directly.
- When implementing or modifying `BattleRandom` / battle determinism.
- When swapping the PRNG algorithm.

## Examples

```rust
// production: thread-local
let idx = emukc_crypto::rng::usize(0..fleet.len());

// deterministic replay: seeded
let mut rng = GameRng::seeded(seed);
let v = rng.i64_inclusive(3..=6);
```

## Related

- `docs/solutions/architecture-patterns/battle-crate-docs.md` — the RNG-cross-phase-continuity doc on `simulate_day`.
- `crates/emukc_gameplay/src/game/battle/rng.rs` — `BattleRng` usage.
- `crates/emukc_crypto/src/rng.rs` — the facade.
