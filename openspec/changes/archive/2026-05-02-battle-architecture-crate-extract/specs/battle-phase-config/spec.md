## ADDED Requirements

### Requirement: Phase sequence configuration
The battle simulation SHALL define a `BattlePhaseKind` enum enumerating all possible battle phases. Each battle type SHALL have an associated ordered phase sequence represented as a `BattleFlow` constant. The orchestrator SHALL dispatch phases in the order declared by the flow, executing each phase exactly once per pass through the sequence.

#### Scenario: Surface day battle has standard phase order
- **WHEN** `BattleFlow::for_type(BattleType::Normal)` is called
- **THEN** the returned sequence SHALL be `[Kouku, OpeningAsw, OpeningTorpedo, Engagement, Shelling1, Shelling2, ClosingTorpedo]`

#### Scenario: Air battle has only aerial phase
- **WHEN** `BattleFlow::for_type(BattleType::AirBattle)` is called
- **THEN** the returned sequence SHALL be `[Kouku]`
- **THEN** no shelling or torpedo phases SHALL execute

#### Scenario: Unknown phase kind is a no-op
- **WHEN** the orchestrator encounters a `BattlePhaseKind` variant that is valid but has no implementation for the current battle type
- **THEN** the orchestrator SHALL skip it without error (reserved for future phases like LBAS, support)

### Requirement: Phase dispatch uses compile-time match
The orchestrator SHALL dispatch phases via a `match` statement on `BattlePhaseKind`, NOT via trait objects or dynamic dispatch. Phase functions SHALL be plain functions that accept `&Codex`, `&mut BattleState`, and `&mut impl BattleRng`.

#### Scenario: New phase requires compiler-verified coverage
- **WHEN** a developer adds a new variant to `BattlePhaseKind` without adding a corresponding `match` arm in the orchestrator
- **THEN** the compiler SHALL produce an error for the non-exhaustive match

#### Scenario: Phase functions receive unified state
- **WHEN** a phase function executes
- **THEN** it SHALL receive `&mut BattleState` containing the current ship states, phase outputs, and protocol flags
- **THEN** it SHALL NOT directly access or modify state belonging to other phases except through the shared `BattleState`

### Requirement: Battle state aggregate lifecycle
Each battle simulation SHALL create a `BattleState` aggregate from `BattleContext`, mutate it through the phase sequence, and finalize it into `BattleSimulation` via a single consumption method. No intermediate state SHALL be observable outside the simulation.

#### Scenario: State is created from context
- **WHEN** `BattleState::from_context(ctx)` is called
- **THEN** `friendly` ships SHALL be constructed with `is_friendly = true` and `is_sortie` from the context
- **THEN** `enemy` ships SHALL be constructed with `is_friendly = false` and `is_sortie` from the context

#### Scenario: State is consumed by finalize
- **WHEN** `state.finalize()` is called after all phases
- **THEN** the method SHALL consume `self` and return `BattleSimulation`
- **THEN** `verify_protected_ships_alive()` SHALL be called before returning
- **THEN** the returned `BattleSimulation` SHALL contain populated `packet` and `outcome` fields

#### Scenario: Phase outputs accumulate in state
- **WHEN** the kouku phase completes
- **THEN** `state.kouku` SHALL be `Some(BattleKouku { ... })`
- **THEN** `state.air_state` SHALL be `Some(AirState)` for subsequent phases to consume
