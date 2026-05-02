## MODIFIED Requirements

### Requirement: Battle deterministic replay

The battle simulation SHALL accept an injected `&mut dyn BattleRng` (or `&mut impl BattleRng`) at every entry point in `crates/emukc_battle`. Two implementations SHALL exist: `SeededRng` (test-only, behind `cfg(test)` or a `testing` feature) for deterministic replay, and `ProductionRng` (non-cryptographic, backed by `emukc_crypto::rng` which wraps `fastrand`) for the running server. The production implementation SHALL NOT carry the name `CryptoRng`, since it does not provide cryptographic guarantees, and its docstring SHALL state explicitly that it is non-cryptographic.

#### Scenario: Seeded battle determinism

- **WHEN** a battle is run with a `SeededRng::new(N)` injected through the orchestration layer
- **THEN** the same seed SHALL always produce the same battle outcome (damage values, target selections, win rank)

#### Scenario: Production RNG fallback

- **WHEN** a battle is run on the live server with a `ProductionRng` injected through the orchestration layer
- **THEN** random decisions SHALL be drawn from `emukc_crypto::rng` thread-local state
- **THEN** the docstring of `ProductionRng` SHALL include the substring "non-cryptographic"

#### Scenario: No internal RNG construction in simulation

- **WHEN** `simulate_day(...)` or `simulate_night(...)` executes
- **THEN** it SHALL NOT call `BattleRandom::new(None)`, `fastrand::Rng::new`, or any other RNG constructor
- **THEN** every random decision SHALL come from the caller-supplied `BattleRng`

## ADDED Requirements

### Requirement: choose_index returns Option for empty inputs

`BattleRng::choose_index` SHALL have the signature `fn choose_index(&mut self, len: usize) -> Option<usize>`. When `len == 0` the method SHALL return `None`. When `len > 0` the method SHALL return `Some(i)` where `i < len`. The trait SHALL NOT rely on `debug_assert!` to guard against zero-length inputs in release mode.

#### Scenario: Empty input returns None

- **WHEN** `rng.choose_index(0)` is called on any `BattleRng` implementation
- **THEN** the call SHALL return `None`
- **THEN** the call SHALL NOT panic in either debug or release builds
- **THEN** the call SHALL NOT consume randomness from the underlying RNG state

#### Scenario: Non-empty input returns valid index

- **WHEN** `rng.choose_index(n)` is called with `n > 0`
- **THEN** the call SHALL return `Some(i)` with `i < n`
- **THEN** for `SeededRng::new(seed)`, the value `i` SHALL be deterministic for the given seed

### Requirement: roll_scratch_damage uses trait default only

The `BattleRng::roll_scratch_damage` method SHALL exist exclusively as a default implementation on the trait. Concrete implementations (`SeededRng`, `ProductionRng`) SHALL NOT override it.

#### Scenario: ProductionRng inherits trait default

- **WHEN** `cargo doc --workspace --no-deps` is generated
- **THEN** `ProductionRng::roll_scratch_damage` SHALL appear as inherited from the trait, not as a concrete override
- **THEN** removing the trait default body SHALL break compilation of every battle module that calls `roll_scratch_damage`
