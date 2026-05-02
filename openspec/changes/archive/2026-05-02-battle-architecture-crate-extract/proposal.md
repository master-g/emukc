## Why

`crates/emukc_gameplay/src/game/battle/core.rs` is a 4,600-line god object containing ~110 functions, 72 unit tests, all battle types/enums, damage calculation, aerial combat, shelling, torpedo phases, night battle, target selection, and special attacks — all in a single file. This monolith makes it impossible to test phases in isolation, causes long compile times, and creates friction for adding missing KanColle mechanics (artillery spotting, CVCI, night CI, LBAS, combined fleet, support expeditions, friend fleet). The current architecture also mixes pure simulation logic with session management and HTTP response building in `sortie.rs`/`practice.rs`, making boundaries unclear. Extracting the battle simulation engine into an independent crate with a clear API boundary, explicit phase configuration, and dependency-injected RNG is prerequisite for all future battle feature work.

## What Changes

- **Extract `emukc_battle` standalone crate**: Pure battle simulation engine with zero database or I/O dependencies. Depends only on `emukc_model` (Codex, API types).
- **Introduce `BattleState` aggregate root**: Centralizes all mutable battle state (ships, phase outputs, flags) that was previously scattered as ~15 local variables across `simulate_day_battle_v1`.
- **Introduce `BattleFlow` phase configuration**: Explicit ordered phase sequences per battle type (`BattlePhaseKind` enum), replacing inline `matches!` guards. New battle types (combined fleet, LBAS) add new flow constants without modifying the orchestrator logic.
- **Introduce `BattleRng` trait**: RNG as a dependency-injected port. Tests inject deterministic seeded RNG; production injects crypto RNG. Removes the `BattleContext.rng_seed` coupling.
- **Introduce `SortieRepository` trait**: Synchronous repository trait for SortieStore access, making the dependency explicit in `HasContext`. Tests can inject alternative implementations.
- **Split session layer**: `battle/sortie.rs` → `battle/sortie/orchestrate.rs` + `battle/sortie/response.rs`. `battle/practice.rs` → `battle/practice/orchestrate.rs` + `battle/practice/exp.rs` + `battle/practice/response.rs`. Separates simulation orchestration from API response building.
- **Phase module structure**: Split `core.rs` into `types.rs`, `damage.rs`, `targeting.rs`, `outcome.rs`, `config.rs`, `state.rs`, `random.rs`, `simulation/` (orchestrator), and `simulation/{kouku,asw,torpedo,shelling,night}.rs`. No behavior changes — pure structural refactor.
- **Orchestrator uses `match` dispatch** (not trait objects): Compile-time exhaustive matching on `BattlePhaseKind` enum. Zero-cost, compiler-verified, explicit data flow between phases.

## Capabilities

### New Capabilities

- `battle-crate`: Standalone battle simulation crate (`emukc_battle`) with public API (`simulate_day`, `simulate_night`, all types), private phase internals, zero I/O dependencies. Compiles independently of `emukc_db` and `emukc_gameplay`.
- `battle-phase-config`: Explicit phase ordering configuration via `BattlePhaseKind` enum and `BattleFlow` constants. Each battle type declares its phase sequence as compile-time data. Adding a new phase means: (1) add enum variant, (2) implement phase function, (3) add to relevant `BattleFlow`, (4) add `match` arm in orchestrator.
- `battle-rng-port`: `BattleRng` trait as the RNG dependency injection port. Two implementations: `CryptoRng` (wraps `emukc_crypto`, production) and `SeededRng` (deterministic, tests). Removes `BattleContext.rng_seed`.
- `sortie-repository`: `SortieRepository` trait for explicit SortieStore dependency. `GlobalSortieStore` as default implementation. Enables test isolation without global mutable state.

### Modified Capabilities

- `sortie`: Session layer structure changes — `battle/sortie.rs` and `battle/practice.rs` split into orchestrate/response sub-modules. `HasContext::sortie_store()` signature changes to return `&dyn SortieRepository` (no default impl). `SortieOps` trait API unchanged — only internal implementation structure changes.

## Impact

- **New crate**: `crates/emukc_battle/` — 14+ files, depends on `emukc_model`, `emukc_crypto`
- **Deleted**: `crates/emukc_gameplay/src/game/battle/core.rs` (4,600 lines, replaced by `emukc_battle`)
- **Modified**: `crates/emukc_gameplay/src/game/battle/sortie.rs` → `battle/sortie/{mod,orchestrate,response}.rs`
- **Modified**: `crates/emukc_gameplay/src/game/battle/practice.rs` → `battle/practice/{mod,orchestrate,exp,response}.rs`
- **Modified**: `crates/emukc_gameplay/src/gameplay.rs` — `HasContext` signature change for `sortie_store()`
- **Modified**: `crates/emukc_gameplay/src/game/battle/mod.rs` — updated module declarations/re-exports
- **Modified**: `crates/emukc_gameplay/src/game/sortie.rs` — import path updates, `SortieRepository` usage
- **Modified**: `crates/emukc_gameplay/src/game/practice.rs` — import path updates
- **Modified**: `crates/emukc_gameplay/src/game/sortie_store.rs` — `SortieRepository` trait impl for `SortieStore`
- **Modified**: `crates/emukc_gameplay/Cargo.toml` — add `emukc_battle` dependency
- **Modified**: `Cargo.toml` (workspace) — add `emukc_battle` member
- **No behavior changes** — all existing tests must pass without modification after migration

## Non-goals

- Changing damage formulas or any battle behavior (separate: `fix-battle-attack-system`)
- Adding new battle phases (LBAS, combined fleet, support) — this creates the architecture to enable them
- Refactoring sortie/practice API handlers (`src/bin/net/router/kcsapi/`)
- Changing the `fix-battle-attack-system` or `refactor-battle-phases` changes (those rebase onto this)
