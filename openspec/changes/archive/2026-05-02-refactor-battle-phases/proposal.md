## Why

`core.rs` is a 4,600-line god object containing ~110 non-test functions (plus 72 unit tests) covering all battle logic — damage calculation, aerial combat, shelling, torpedo phases, night battle, target selection, and special attacks. This makes the file unmaintainable: any change risks regressions, testing individual phases in isolation is difficult, and the existing `fix-battle-attack-system` change is already painful to apply on top of this monolith. Splitting into focused modules is prerequisite for adding missing KanColle mechanics (artillery spotting, CVCI, night CI, LBAS, combined fleet).

## What Changes

- Split `crates/emukc_gameplay/src/game/battle/core.rs` into phase-oriented modules under `battle/`
- Extract shared types (`BattleContext`, `BattleRuntimeShip`, etc.) into `battle/types.rs`
- Extract damage calculation pipeline into `battle/damage.rs`
- Extract target selection into `battle/targeting.rs`
- Extract aerial combat into `battle/phases/kouku.rs`
- Extract shelling into `battle/phases/shelling.rs`
- Extract torpedo into `battle/phases/torpedo.rs`
- Extract night battle into `battle/phases/night.rs`
- Extract OASW into `battle/phases/asw.rs`
- Extract orchestrator entry points (`simulate_day_battle_v1`, `simulate_night_battle_v1`) into `battle/simulation.rs`
- Extract outcome functions (`calculate_mvp`, `calculate_win_rank`, `verify_protected_ships_alive`) into `battle/outcome.rs`
- Re-export all public items from `battle/mod.rs` so external callers unchanged
- No behavior changes — pure structural refactor

## Capabilities

### New Capabilities

### Modified Capabilities

- `battle-damage-foundation`: No requirement change, but implementation file moves from `core.rs` to `battle/damage.rs`
- `battle-kouku-stage3`: No requirement change, implementation moves to `battle/phases/kouku.rs`
- `sortie`: Day battle simulation requirement unchanged, implementation references updated

## Impact

- `crates/emukc_gameplay/src/game/battle/core.rs` — deleted, replaced by module tree
- `crates/emukc_gameplay/src/game/battle/mod.rs` — updated module declarations
- `crates/emukc_gameplay/src/game/battle/sortie.rs` — import path updates only
- `crates/emukc_gameplay/src/game/battle/practice.rs` — import path updates only
- No API handler changes, no database changes, no model changes
- All existing tests must pass without modification

## Non-goals

- Adding new battle mechanics (separate changes)
- Changing damage formulas or battle behavior in any way
- Modifying the `fix-battle-attack-system` change (that proceeds independently)
- Refactoring sortie/practice handler files
- Changing public trait signatures
