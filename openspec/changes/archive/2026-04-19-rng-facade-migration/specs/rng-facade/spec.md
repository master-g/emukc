## ADDED Requirements

### Requirement: GameRng seeded instance
The system SHALL provide a `GameRng` type that can be created from a `u64` seed, producing deterministic output sequences for a given seed.

#### Scenario: Create seeded RNG
- **WHEN** `GameRng::seeded(seed)` is called with a specific `u64` value
- **THEN** the returned `GameRng` SHALL produce identical output sequences across multiple invocations with the same seed

#### Scenario: Seeded RNG integer range
- **WHEN** `game_rng.i64(min..max)` is called on a seeded instance
- **THEN** it SHALL return a value in `[min, max)` deterministically based on the seed

### Requirement: Inclusive integer range support
The facade SHALL accept `RangeInclusive` bounds (`min..=max`) in addition to exclusive ranges, abstracting the backend's exclusive-only limitation.

#### Scenario: Thread-local inclusive range
- **WHEN** `rng::i64_inclusive(3..=6)` is called
- **THEN** it SHALL return a value in `[3, 6]` (inclusive on both ends)

#### Scenario: Seeded inclusive range
- **WHEN** `game_rng.i64_inclusive(0..=3)` is called on a seeded instance
- **THEN** it SHALL return a value in `[0, 3]` deterministically based on the seed

#### Scenario: Inclusive range at type maximum
- **WHEN** `rng::i64_inclusive(i64::MAX..=i64::MAX)` is called
- **THEN** it SHALL return `i64::MAX` without overflow (saturating conversion)

### Requirement: Float range support
The facade SHALL provide `f64_range(min, max)` returning a random `f64` in `[min, max)`.

#### Scenario: Thread-local float range
- **WHEN** `rng::f64_range(0.0, 100.0)` is called
- **THEN** it SHALL return a value in `[0.0, 100.0)`

#### Scenario: Seeded float range
- **WHEN** `game_rng.f64_range(0.3, 1.5)` is called on a seeded instance
- **THEN** it SHALL return a value in `[0.3, 1.5)` deterministically based on the seed

### Requirement: Thread-local RNG free functions
The system SHALL provide free functions (`usize`, `i64`, `u32`, `f64`) that return random values from thread-local state without requiring an explicit RNG instance.

#### Scenario: Thread-local integer range
- **WHEN** `rng::usize(0..len)` is called
- **THEN** it SHALL return a `usize` value in `[0, len)` drawn from thread-local RNG state

#### Scenario: Thread-local float
- **WHEN** `rng::f64()` is called
- **THEN** it SHALL return an `f64` value in `[0.0, 1.0)`

### Requirement: Collection randomization
The system SHALL provide `shuffle`, `choose`, and `choose_iter` helpers.

#### Scenario: Shuffle slice
- **WHEN** `rng::shuffle(&mut slice)` is called
- **THEN** the slice elements SHALL be randomly permuted in place

#### Scenario: Choose from non-empty slice
- **WHEN** `rng::choose(slice)` is called on a non-empty slice
- **THEN** it SHALL return `Some(&element)` with uniformly random selection

#### Scenario: Choose from empty slice
- **WHEN** `rng::choose(&[])` is called
- **THEN** it SHALL return `None`

#### Scenario: Choose from iterator
- **WHEN** `rng::choose_iter(iter)` is called on a non-empty iterator
- **THEN** it SHALL return `Some(&element)` with uniformly random selection

#### Scenario: Choose from empty iterator
- **WHEN** `rng::choose_iter(std::iter::empty())` is called
- **THEN** it SHALL return `None`

### Requirement: Single-point backend encapsulation
The RNG backend SHALL be encapsulated in `emukc_crypto::rng` so that swapping the PRNG algorithm requires modifying only that module.

#### Scenario: Backend swap isolation
- **WHEN** the RNG backend (e.g., fastrand → biski64) is changed in `emukc_crypto::rng`
- **THEN** no other crate or file SHALL require modification for the change to take effect

### Requirement: Battle deterministic replay
`BattleRandom` in `crates/emukc_gameplay/src/game/battle/core.rs` SHALL use `GameRng` for seeded mode, preserving deterministic battle replay.

#### Scenario: Seeded battle determinism
- **WHEN** a battle is run with `rng_seed: Some(N)`
- **THEN** the same seed SHALL always produce the same battle outcome

#### Scenario: Unseeded battle fallback
- **WHEN** a battle is run with `rng_seed: None`
- **THEN** the system SHALL use thread-local RNG free functions for random values
