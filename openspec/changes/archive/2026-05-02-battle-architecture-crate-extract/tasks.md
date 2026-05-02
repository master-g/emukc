## 1. Scaffold emukc_battle crate

- [x] 1.1 Create `crates/emukc_battle/` directory with `Cargo.toml` depending on `emukc_model`, `emukc_crypto`
- [x] 1.2 Add `emukc_battle` to workspace `Cargo.toml` members
- [x] 1.3 Create `lib.rs` — declare all modules, re-export public API items
- [x] 1.4 Create `types.rs` — copy all structs, enums, constants from `core.rs` (~800 lines)
- [x] 1.5 Create `damage.rs` — copy all damage calculation functions from `core.rs`
- [x] 1.6 Create `targeting.rs` — copy all targeting and eligibility functions from `core.rs`
- [x] 1.7 Create `outcome.rs` — copy `calculate_mvp`, `calculate_win_rank`, `verify_protected_ships_alive`
- [x] 1.8 Create `simulation/` directory with `mod.rs` and phase files (kouku, asw, torpedo, shelling, night)
- [x] 1.9 Copy `simulate_day_battle_v1` → `simulation/mod.rs` as `simulate_day()`
- [x] 1.10 Copy `simulate_night_battle_v1` → `simulation/mod.rs` as `simulate_night()`
- [x] 1.11 Move `BattleRandom` struct into crate (keep as private)
- [x] 1.12 Copy `#[cfg(test)] mod tests` block (72 tests + helpers) into `tests/` directory — one test file per logical area
- [x] 1.13 **Gate**: `cargo check -p emukc_battle` — compiles (tests deferred to Phase 2)

## 2. Wire emukc_battle into emukc_gameplay

- [x] 2.1 Add `emukc_battle` dependency to `emukc_gameplay/Cargo.toml`
- [x] 2.2 Update `battle/sortie.rs` imports from `super::core::` to `emukc_battle::`
- [x] 2.3 Update `battle/practice.rs` imports from `super::core::` to `emukc_battle::`
- [x] 2.4 Update `game/sortie.rs` imports from `battle::core::` to `emukc_battle::`
- [x] 2.5 Update `game/sortie_result.rs` reference from `super::battle::core::BattleShipInput`
- [x] 2.6 Delete `crates/emukc_gameplay/src/game/battle/core.rs`
- [x] 2.7 **Gate**: `cargo check --workspace` — all crates compile
- [x] 2.8 **Gate**: `cargo test --test gameplay_tests` — integration tests pass
- [x] 2.9 **Gate**: `cargo test -p emukc_gameplay` — unit tests pass

## 3. Introduce BattleState + BattleFlow (Phase 3 in migration plan)

- [x] 3.1 Create `emukc_battle/src/state.rs` — `BattleState` struct with all mutable fields
- [x] 3.2 Implement `BattleState::from_context(BattleContext) -> Self`
- [x] 3.3 Implement `BattleState::finalize(self) -> BattleSimulation` (verify sinking protection, build packet + outcome)
- [x] 3.4 Create `emukc_battle/src/config.rs` — `BattlePhaseKind` enum, `BattleFlow` struct with `&'static [BattlePhaseKind]`
- [x] 3.5 Define `BattleFlow: SURFACE_DAY`, `AIR_BATTLE`, `SURFACE_DAY_NO_TORPEDO` constants
- [x] 3.6 Refactor `simulation/mod.rs` orchestrator: replace inline `if run_X` blocks with `for &phase in flow { match phase { ... } }`
- [x] 3.7 Update all phase functions to accept `&mut BattleState` instead of individual `&mut` parameters
- [x] 3.8 Update `simulate_night()` to use `BattleState` for host-side state
- [x] 3.9 **Gate**: all existing tests pass (behavior unchanged)
- [x] 3.10 **Gate**: `cargo clippy --workspace` — no new warnings

## 4. BattleRng trait

- [x] 4.1 Define `BattleRng` trait in `emukc_battle/src/random.rs` with 4 methods: `choose_index`, `roll_scratch_damage`, `random_f64_range`, `roll_range`
- [x] 4.2 Update `simulate_day()` signature: `fn simulate_day(codex: &Codex, context: BattleContext, rng: &mut impl BattleRng) -> BattleSimulation`
- [x] 4.3 Update `simulate_night()` signature: same pattern
- [x] 4.4 Update all phase functions to accept `rng: &mut impl BattleRng`
- [x] 4.5 Remove `BattleRandom` struct
- [x] 4.6 Remove `BattleContext.rng_seed` field
- [x] 4.7 Create `SeededRng` in `emukc_battle/src/random.rs` — wraps deterministic `GameRng`
- [x] 4.8 Create `CryptoRng` in `emukc_gameplay/src/game/battle/rng.rs` — wraps `emukc_crypto::rng`
- [x] 4.9 Update all callers: `simulate_and_store_sortie_day_battle` creates `CryptoRng`, tests create `SeededRng`
- [x] 4.10 **Gate**: `cargo test --workspace` — all tests pass with seeded RNG

## 5. SortieRepository trait

- [x] 5.1 Define `SortieRepository` trait in `emukc_gameplay/src/game/battle/repository.rs` — 9 synchronous methods
- [x] 5.2 Implement `SortieRepository` for `SortieStore` (production)
- [x] 5.3 Create `TestSortieStore` implementing `SortieRepository` for test isolation
- [x] 5.4 Update `HasContext` trait: change `fn sortie_store(&self) -> &SortieStore` to `fn sortie_store(&self) -> &dyn SortieRepository`, remove default impl
- [x] 5.5 Update all four `impl HasContext for (...)` blocks to provide explicit `sortie_store()`
- [x] 5.6 Update session functions (`simulate_and_store_sortie_day_battle`, etc.) to accept `&dyn SortieRepository` instead of `&SortieStore`
- [x] 5.7 Update `game/sortie.rs` callers to use `SortieRepository` methods
- [x] 5.8 **Gate**: `cargo check --workspace` compiles
- [x] 5.9 **Gate**: `cargo test --test gameplay_tests` — integration tests pass (isolated stores)

## 6. Session layer split

- [x] 6.1 Move `battle/sortie.rs` → `battle/sortie/mod.rs` + `orchestrate.rs` + `response.rs`
- [x] 6.2 `orchestrate.rs`: `run_day_battle()`, `run_night_battle()`, `run_sp_midnight_battle()` — pure orchestration (build context → call emukc_battle → persist)
- [x] 6.3 `response.rs`: `build_day_response()`, `build_night_response()` — API response construction only
- [x] 6.4 Move `battle/practice.rs` → `battle/practice/mod.rs` + `orchestrate.rs` + `response.rs` + `exp.rs`
- [x] 6.5 `practice/orchestrate.rs`: `run_day_battle()`, `run_night_battle()` — orchestration
- [x] 6.6 `practice/exp.rs`: `calculate_admiral_exp()`, `calculate_ship_exp()` — XP calculation only
- [x] 6.7 `practice/response.rs`: response builders
- [x] 6.8 Update `battle/mod.rs` module declarations
- [x] 6.9 **Gate**: `cargo check --workspace` compiles
- [x] 6.10 **Gate**: `cargo test --workspace` — all tests pass

## 7. Cleanup and final verification

- [x] 7.1 Remove any dead code or unused imports introduced during migration
- [x] 7.2 Run `cargo fmt --all`
- [x] 7.3 Run `cargo clippy --workspace` — fix any warnings
- [x] 7.4 Run `cargo test --workspace` — full test suite must pass
- [x] 7.5 Run `cargo test --test gameplay_tests` — integration gameplay tests must pass
- [x] 7.6 Verify `emukc_battle` compiles independently: `cargo check -p emukc_battle` without building gameplay
