## Context

The battle simulation code currently lives entirely in `crates/emukc_gameplay/src/game/battle/core.rs` — a 4,600-line file containing all types, phases, damage calculations, target selection, night battle logic, special attack systems, and 72 unit tests. Two session-layer files (`sortie.rs` 282 lines, `practice.rs` 677 lines) call into `core.rs`, handling SortieStore persistence, API response building, and experience calculation.

The existing gameplay crate follows a consistent pattern: domain traits (`ShipOps`, `MaterialOps`, etc.) with blanket implementations on `HasContext`, backed by `_impl` functions taking generic `C: ConnectionTrait`. Complex domains like `compose/` and `quest/` already split into sub-modules organized by operation or concern. Battle code has not followed this pattern — `core.rs` is the largest single file in the entire workspace.

External callers referencing `core.rs`:
- `battle/sortie.rs` — imports `BattleContext`, `BattlePacket`, `BattleRuntimeShip`, `BattleSimulation`, `EngagementType`, `NightBattlePacket`, `simulate_day_battle_v1`, `simulate_night_battle_v1`
- `battle/practice.rs` — imports most public types + both simulate functions
- `game/sortie.rs` — imports `BattleContext`, `BattlePacket`, `BattleShipInput`, `BattleType`, `EngagementType`, `BattleNightHougeki`

All current types in `core.rs` are either `pub` (within the `pub(crate)` module) or private. `BattleRandom` is private (not `pub`). The module itself is declared `pub(crate)` in `battle/mod.rs`.

The architecture audit identified:
1. `core.rs` lacks phase isolation — all 110+ functions share a single namespace
2. Phase dispatch uses inline `matches!` guards that make adding new battle types brittle
3. RNG is coupled to `BattleContext.rng_seed` (optional seed for tests, `None` for production)
4. `SortieStore` is a global `LazyLock<SortieStore>` accessed implicitly through `HasContext`
5. Session layer (`sortie.rs`/`practice.rs`) mixes three concerns: orchestration, persistence, and response building

## Goals / Non-Goals

**Goals:**
- Extract battle simulation engine into `emukc_battle` standalone crate with zero I/O dependencies
- Centralize mutable battle state into a `BattleState` aggregate root
- Formalize phase sequence configuration as `BattleFlow` with `BattlePhaseKind` enum
- Define `BattleRng` trait for dependency-injected randomness
- Define `SortieRepository` trait for explicit SortieStore dependency
- Split session layer into orchestrate/response sub-modules
- All existing tests pass without modification

**Non-Goals:**
- Adding new battle mechanics or phases
- Changing any battle behavior, damage formulas, or phase logic
- Refactoring API handlers (`src/bin/net/router/kcsapi/`)
- Replacing `SortieStore` with database persistence
- Making `SortieRepository` async

## Decisions

### D1: Crate boundary — `emukc_battle` as a standalone crate

**Decision**: Create `crates/emukc_battle/` as a new workspace member with dependencies only on `emukc_model` and `emukc_crypto` (for RNG types). No dependency on `emukc_db`, `emukc_gameplay`, or `emukc_bootstrap`.

**Public API** (`pub` items exported from `lib.rs`):
- Entry functions: `simulate_day()`, `simulate_night()`
- Input types: `BattleContext`, `BattleShipInput`, `BattleType`, `EngagementType`
- Output types: `BattleSimulation`, `NightBattleSimulation`, `BattleOutcome`, `BattlePacket`, `NightBattlePacket`
- Intermediate: `BattleRuntimeShip` (needed by session layer for HP state tracking)
- Utility: `apply_cap()`, `any_alive()`, `calculate_mvp()`, `calculate_win_rank()`
- Port: `BattleRng` trait
- Phase config: `BattlePhaseKind` enum

**Private API** (`pub(crate)` or private):
- `BattleState` aggregate (internal state machine)
- `BattleFlow` structs and constants
- All phase functions (`simulate_kouku`, `simulate_shelling_side`, etc.)
- `damage.rs`, `targeting.rs`, `outcome.rs` internal helpers
- `BattleRandom` struct (replaced by `BattleRng` trait later)

**Rationale**: Battle simulation is computationally pure — it takes `Codex` (read-only) and `BattleContext` (owned input) and produces `BattleSimulation` (owned output). No database, no HTTP, no side effects. This makes it a natural crate boundary. Independent compilation speeds up iteration. Future tools (battle CLI diagnostics, test harness) can depend on `emukc_battle` without pulling in the full gameplay stack.

**Dependency direction in workspace**:
```
emukc_gameplay ──► emukc_battle ──► emukc_model ──► emukc_crypto
```

### D2: Phase dispatch — `match` on `BattlePhaseKind` enum

**Decision**: Use compile-time `match` on a `BattlePhaseKind` enum rather than `dyn trait BattlePhase` with trait objects or generic trait dispatch.

```rust
pub enum BattlePhaseKind {
    Kouku, OpeningAsw, OpeningTorpedo, Engagement,
    Shelling1, Shelling2, ClosingTorpedo, NightBattle,
}

// In orchestrator:
for &phase_kind in flow.phases {
    match phase_kind {
        BattlePhaseKind::Kouku => phases::kouku::execute(codex, &mut state, rng),
        BattlePhaseKind::OpeningAsw => phases::asw::execute(codex, &mut state, rng),
        BattlePhaseKind::Shelling1 => phases::shelling::execute_round1(codex, &mut state, rng),
        // ...
    }
}
```

**Rationale**: Phase functions share mutable state (`BattleState`) and have data dependencies between them (kouku produces `air_state` consumed by shelling; shelling modifies HP consumed by torpedo targeting). A trait-based approach would obscure these dependencies behind a uniform interface that pretends phases are independent. The `match` approach is:
- Zero-cost (direct function calls, no vtable indirection)
- Exhaustive (compiler verifies all enum variants are handled)
- Explicit about data flow between phases
- Adding a new phase = add enum variant + implement function + add `match` arm + add to `BattleFlow` configs

**Alternatives considered**:
- `trait BattlePhase { fn execute(&self, state: &mut BattleState, rng: &mut dyn BattleRng) }` — rejected because phases don't share a common interface (different parameter needs), and `Box<dyn BattlePhase>` introduces vtable overhead
- `enum_dispatch` crate — rejected as unnecessary dependency; the `match` approach provides the same compile-time dispatch without external crates

### D3: Aggregate root — `BattleState`

**Decision**: Replace the ~15 local variables scattered across `simulate_day_battle_v1` (150+ lines of `let mut`) with a single `BattleState` struct that owns all mutable state for one battle simulation.

```rust
pub(crate) struct BattleState {
    pub friendly: Vec<BattleRuntimeShip>,
    pub enemy: Vec<BattleRuntimeShip>,
    pub is_sortie: bool,
    pub friendly_formation: i64,
    pub enemy_formation: i64,
    pub engagement: EngagementType,
    // Phase outputs (accumulated)
    pub kouku: Option<BattleKouku>,
    pub opening_attack: Option<BattleOpeningAttack>,
    pub opening_taisen: Option<BattleHougeki>,
    pub hougeki1: Option<BattleHougeki>,
    pub hougeki2: Option<BattleHougeki>,
    pub raigeki: Option<BattleRaigeki>,
    // Inter-phase state
    pub air_state: Option<AirState>,
    // Protocol flags
    pub stage_flag: [i64; 3],
    pub hourai_flag: [i64; 4],
    pub opening_taisen_flag: i64,
}
```

**Lifecycle**: `BattleState::from_context(BattleContext)` → mutated by phases → `state.finalize()` → produces `BattleSimulation`.

**Rationale**: The battle simulation is a single transaction. All phases operate on the same ships, flags, and accumulated outputs. Grouping them into a single struct makes the data flow explicit, eliminates accidental uninitialized state, and provides a single consumption point (`finalize()`) that guarantees invariants (sinking protection verified, all required fields populated).

### D4: RNG port — `BattleRng` trait

**Decision**: Define a `BattleRng` trait and accept `&mut impl BattleRng` in the orchestrator. Remove `BattleContext.rng_seed`.

```rust
pub trait BattleRng {
    fn choose_index(&mut self, len: usize) -> usize;
    fn roll_scratch_damage(&mut self, current_hp: i64) -> i64;
    fn random_f64_range(&mut self, min: f64, max: f64) -> f64;
    fn roll_range(&mut self, min: i64, max: i64) -> i64;
}
```

**Implementations**:
- `CryptoRng` (in `emukc_gameplay`): wraps `emukc_crypto::rng::*` functions for production use
- `SeededRng` (in `emukc_battle::tests`): deterministic, wraps a seeded `GameRng` for test reproducibility

**Rationale**: The current `BattleRandom` internally checks `self.seeded` to dispatch between seeded and global RNG. This conflates test and production concerns in the same struct. A trait extracts the interface, making the caller responsible for choosing the right implementation. The current `rng_seed: Option<u64>` on `BattleContext` is removed — callers pass the appropriate RNG implementation.

**Impact on callers**:
- `simulate_day()` signature becomes `simulate_day(codex, context, rng)` instead of `simulate_day(codex, context)` (where context held the seed)
- `simulate_night()` already uses `BattleRandom::new(None)` — changes to accept `&mut impl BattleRng`

### D5: Repository — `SortieRepository` trait

**Decision**: Define a synchronous `SortieRepository` trait. `HasContext` returns `&dyn SortieRepository` instead of `&SortieStore`. No default implementation.

```rust
pub trait SortieRepository: Send + Sync {
    fn get_active(&self, profile_id: i64) -> Option<ActiveSortieState>;
    fn insert_active(&self, profile_id: i64, state: ActiveSortieState) -> Option<ActiveSortieState>;
    fn remove_active(&self, profile_id: i64);
    fn get_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession>;
    fn insert_pending_battle(&self, profile_id: i64, session: SortieBattleSession);
    fn take_pending_battle(&self, profile_id: i64) -> Option<SortieBattleSession>;
    fn get_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot>;
    fn insert_pending_result(&self, profile_id: i64, result: SortieBattleResultSnapshot);
    fn take_pending_result(&self, profile_id: i64) -> Option<SortieBattleResultSnapshot>;
}
```

**Rationale**: The current `HasContext::sortie_store()` returns `&GLOBAL_SORTIE_STORE` as a default implementation. This hides the dependency and makes test isolation impossible (tests share global mutable state). Making the trait explicit in `HasContext` forces callers to provide an implementation. `GlobalSortieStore` remains as the production default. Tests inject a fresh `TestSortieStore`.

**Why synchronous?** Current code uses `Mutex` internally and performs no async I/O. Introducing `#[async_trait]` would propagate async through all callers without benefit. The trait stays synchronous.

## Risks / Trade-offs

- **[Risk] Crate boundary exposes previously `pub(crate)` types**: Types like `BattleKouku`, `BattleHougeki`, `BattleRaigeki` must become `pub` in `emukc_battle`. These were previously crate-private. Mitigation: they are data types (no invariants to protect), and making them public is no worse than exposing them through re-exports.
- **[Risk] `BattleRng` trait monomorphization**: Using `impl BattleRng` (not `&dyn BattleRng`) in the orchestrator causes monomorphization of `simulate_day()` for each RNG type. Mitigation: only two implementations exist (`CryptoRng`, `SeededRng`), and `simulate_day` is called once per API request — not in a hot loop. The negligible binary size increase is worth the zero-cost dispatch.
- **[Risk] `SortieRepository` breaks existing `HasContext` impls**: Four tuple implementations of `HasContext` currently rely on the default `sortie_store()` returning `&GLOBAL_SORTIE_STORE`. Mitigation: update all four `impl HasContext for` blocks to provide a concrete `sortie_store()` implementation. The test `HasContext` impls already use an explicit `SortieStore`.
- **[Risk] Merge conflict with `fix-battle-attack-system`**: That change modifies behavior in `core.rs` functions. This change moves those functions to `emukc_battle`. Mitigation: apply this architecture change first (pure structural), then rebase the attack-system fix onto the new crate structure. The attack-system changes are targeted (3-4 functions in `damage.rs` and `targeting.rs`).
- **[Trade-off] `match` dispatch requires editing orchestrator for new phases**: Adding a new phase means adding an arm to the `match` in `simulation/mod.rs`. This is a feature, not a bug — the compiler enforces that all phases are handled. A trait-based approach would allow adding phases without touching the orchestrator, but at the cost of hidden coupling and runtime overhead.

## Migration Plan

### Phase 1: Create `emukc_battle` with structural copy (no logic changes)

1. Scaffold `crates/emukc_battle/` with `Cargo.toml` depending on `emukc_model`
2. Copy `core.rs` content into module files following the target structure:
   - `types.rs` (all structs, enums, constants)
   - `damage.rs` (all damage functions)
   - `targeting.rs` (all targeting/eligibility functions)
   - `outcome.rs` (MVP, win_rank, sinking protection verification)
   - `simulation/mod.rs` (orchestrator: `simulate_day_battle_v1`, `simulate_night_battle_v1`)
   - `simulation/kouku.rs`, `asw.rs`, `torpedo.rs`, `shelling.rs`, `night.rs` (phase functions)
3. `BattleRandom` remains private struct within the crate
4. Update `pub(crate)` visibility to `pub` for API items
5. Add `emukc_battle` to workspace `Cargo.toml`
6. **Gate**: `cargo check -p emukc_battle` compiles cleanly

### Phase 2: Wire into `emukc_gameplay`

1. Add `emukc_battle` dependency to `emukc_gameplay/Cargo.toml`
2. Update imports in `battle/sortie.rs`, `battle/practice.rs`, `game/sortie.rs` to use `emukc_battle::` paths
3. Delete `crates/emukc_gameplay/src/game/battle/core.rs`
4. **Gate**: `cargo check --workspace` compiles cleanly, `cargo test --workspace` passes

### Phase 3: Introduce `BattleState` + `BattleFlow` (structural, no behavior change)

1. Create `state.rs` — `BattleState` struct with `from_context()` and `finalize()`
2. Create `config.rs` — `BattlePhaseKind` enum and `BattleFlow` constants
3. Refactor `simulation/mod.rs` orchestrator to use `for + match` instead of inline `if run_X` blocks
4. Phase functions changed to accept `&mut BattleState` instead of 10+ individual `&mut` parameters
5. **Gate**: all existing tests pass (behavior unchanged)

### Phase 4: `BattleRng` trait

1. Define `BattleRng` trait in `emukc_battle`
2. `simulate_day()` and `simulate_night()` accept `&mut impl BattleRng`
3. Phase functions forwarded the same `rng` parameter
4. Remove `BattleRandom` struct and `BattleContext.rng_seed`
5. Create `SeededRng` in `emukc_battle` test helpers
6. Create `CryptoRng` in `emukc_gameplay::battle::rng`
7. Update callers: build RNG before calling simulate
8. **Gate**: all tests pass with seeded RNG, production code compiles with crypto RNG

### Phase 5: `SortieRepository` trait + Session layer split

1. Define `SortieRepository` trait in `emukc_gameplay`
2. Implement for `SortieStore` (production) and a new `TestSortieStore` (tests)
3. Update `HasContext` — remove default `sortie_store()`, make abstract
4. Update four `impl HasContext` blocks to provide `sortie_store()`
5. Split `battle/sortie.rs` → `battle/sortie/{mod,orchestrate,response}.rs`
6. Split `battle/practice.rs` → `battle/practice/{mod,orchestrate,exp,response}.rs`
7. Session functions accept `&dyn SortieRepository` instead of `&SortieStore`
8. **Gate**: `cargo test --test gameplay_tests` passes

### Rollback strategy

Each phase is independently buildable and testable. If any phase breaks, revert to the previous phase's working state. Phase 2 (delete `core.rs`) is the point of no return — ensure Phase 1 passes before proceeding.

## Open Questions

- Q1: Should `BattleRuntimeShip::apply_damage()` remain a private method on the ship, or become a free function in `damage.rs` that takes `&mut BattleState`? The current method uses `ship_index` for sinking protection (flagship check). If it becomes a free function, the orchestrator passes `ship_index`. Decision deferred to Phase 3 based on actual ergonomics.
- Q2: Should `BattleFlow` constants be `&'static [BattlePhaseKind]` (zero-allocation slice) or `Vec<BattlePhaseKind>` (owned, but allows runtime construction)? Recommendation: `&'static [BattlePhaseKind]` — all known battle types have compile-time-determined phase sequences.
