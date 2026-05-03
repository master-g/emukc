## Why

A systematic audit of `crates/emukc_battle` revealed one correctness bug, one dead-code smell, one undocumented simplification, and one code duplication. The bug (night battle `is_sortie` hardcoded to `false`) silently disables sinking protection (轟沈ストッパー) during night battles in sorties, which violates KanColle game rules. The remaining issues are maintainability concerns that should be resolved while the battle architecture is fresh.

## What Changes

- **Fix night battle `is_sortie` propagation.** `simulate_night` currently constructs a `BattleState` with `is_sortie: false` unconditionally. The caller must supply the sortie context so that sinking protection applies correctly during sortie night battles. **BREAKING**: `NightBattleInput` gains an `is_sortie: bool` field.
- **Remove unused `BattleState.is_sortie` field.** After the fix, `BattleState` no longer needs its own `is_sortie` because the field lives on each `BattleRuntimeShip`. Remove the dead field.
- **Document air Stage2 simplification.** The current kouku Stage2 uses a linear approximation (`total_aa / 400 × plane_count`) instead of per-ship AA with slot-level shootdowns. Add a `// NOTE:` comment explaining the known deviation and that it should be replaced before implementing `AirBattle` / `LdAirBattle` modes fully.
- **Document RNG cross-phase continuity.** Add a doc comment to `simulate_day` explaining that the `rng` parameter is consumed sequentially across all phases, so the same seed produces a deterministic full battle.
- **Deduplicate formation modifiers.** `shelling_formation_modifier` and `torpedo_formation_modifier` in `damage.rs` have identical implementations. Extract a shared `formation_modifier` function.

## Non-goals

- Replacing the air Stage2 linear AA model with a full per-ship AA calculation (that is a separate, larger change).
- Refactoring `BattlePhase` / `BattlePhaseKind` naming (cosmetic, not worth the churn).
- Adding new battle types (联合舰队, etc.) or new CI types.

## Capabilities

### New Capabilities

- `night-battle-sinking-protection`: defines the contract that `simulate_night` SHALL receive sortie context and apply sinking protection correctly.

### Modified Capabilities

- `battle-crate-docs`: existing spec for battle crate documentation — add requirements for RNG continuity docs and Stage2 simplification disclosure.

## Impact

- **Affected crate**: `emukc_battle` (primary), `emukc_gameplay` (callers of `simulate_night`).
- **Public API**: `NightBattleInput` gains `is_sortie: bool`. Breaking for any external consumer; the struct is internal but exported.
- **Tests**: existing night battle tests must be updated to supply `is_sortie`. A new test verifying sinking protection during sortie night battles should be added.
- **No DB schema changes, no Codex changes, no KCSAPI route changes.**
