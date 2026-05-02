## ADDED Requirements

### Requirement: BattleRng trait as RNG dependency injection port
The battle simulation SHALL define a `BattleRng` trait with four methods: `choose_index`, `roll_scratch_damage`, `random_f64_range`, and `roll_range`. The simulation entry points SHALL accept `&mut impl BattleRng` (or `&mut dyn BattleRng`) rather than constructing their own RNG internally. `BattleContext` SHALL NOT carry an `rng_seed` field.

#### Scenario: Caller provides RNG implementation
- **WHEN** `simulate_day(codex, context, rng)` is called
- **THEN** all random decisions during the simulation SHALL use the provided `rng` implementation
- **THEN** the simulation SHALL NOT create its own RNG internally

#### Scenario: Seeded RNG produces deterministic output
- **WHEN** `simulate_day()` is called twice with identical `Codex`, `BattleContext`, and a `SeededRng::new(42)`
- **THEN** both calls SHALL produce bit-identical `BattleSimulation` outputs

#### Scenario: Crypto RNG uses platform entropy
- **WHEN** `simulate_day()` is called with `CryptoRng::new()`
- **THEN** random decisions SHALL use `emukc_crypto::rng` functions (non-deterministic)

#### Scenario: Night battle accepts RNG parameter
- **WHEN** `simulate_night(codex, friendly, enemy, ..., rng)` is called
- **THEN** the simulation SHALL use the provided `rng` instead of internally constructing `BattleRandom::new(None)`

### Requirement: BattleRng implementations
Two implementations of `BattleRng` SHALL exist: `SeededRng` for deterministic testing, and `CryptoRng` for production use. `SeededRng` SHALL reside in `emukc_battle::test_utils` (available only under `#[cfg(test)]` or behind a `testing` feature flag). `CryptoRng` SHALL reside in `emukc_gameplay`.

#### Scenario: SeededRng in test environment
- **WHEN** a unit test creates `SeededRng::new(seed)`
- **THEN** all random operations SHALL produce the same sequence for the same seed
- **THEN** tests can assert on specific damage values and target selections

#### Scenario: CryptoRng in production
- **WHEN** a sortie battle is initiated in the running server
- **THEN** the session layer SHALL create `CryptoRng::new()` and pass it to `simulate_day()`
- **THEN** `BattleContext` SHALL NOT contain an `rng_seed` field
