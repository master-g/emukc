## Context

`crates/emukc_battle` is the pure-computation battle simulation engine. It takes `Codex` (read-only) and battle inputs, produces simulation results. No database, HTTP, or side effects.

The audit found five issues ranging from a correctness bug to code duplication. The most critical: `simulate_night` hardcodes `is_sortie: false` when constructing `BattleState`, which means sinking protection (轟沈ストッパー) is silently disabled during sortie night battles. In real KanColle, night battles occur during sorties and sinking protection applies.

Current code path:
```
emukc_gameplay::battle::sortie::orchestrate::run_night_battle()
  → emukc_battle::simulation::simulate_night(codex, input, rng)
    → BattleState { is_sortie: false, ... }  // BUG
    → BattleRuntimeShip::new(input, is_friendly, is_sortie=false)  // protection disabled
```

## Goals / Non-Goals

**Goals:**
- Fix sinking protection during sortie night battles
- Remove dead `BattleState.is_sortie` field
- Document known air Stage2 simplification
- Document RNG cross-phase continuity
- Deduplicate formation modifier functions

**Non-Goals:**
- Replacing air Stage2 with per-ship AA calculation
- Refactoring `BattlePhase` / `BattlePhaseKind` naming
- Adding new battle types or CI types

## Decisions

### D1. `NightBattleInput` gains `is_sortie: bool`

**Decision**: Add `is_sortie: bool` to `NightBattleInput`. The caller (`emukc_gameplay` orchestrate layer) already knows whether this is a sortie or practice battle and can supply the value directly.

**Alternative considered**: Thread `is_sortie` through `simulate_night` as a separate parameter. Rejected — `NightBattleInput` already bundles all night battle context, and adding a field is more consistent than adding a parameter.

**Alternative considered**: Store `is_sortie` on `BattleState` and propagate from there. Rejected — `BattleState.is_sortie` is currently dead code (never read after construction), and each `BattleRuntimeShip` already carries its own `is_sortie`. Removing the `BattleState` field is cleaner.

### D2. Remove `BattleState.is_sortie` entirely

**Decision**: Delete the `is_sortie` field from `BattleState`. The field was only used to pass the value to `BattleRuntimeShip::new()` during `from_context()`. After D1, the night battle path constructs `BattleRuntimeShip` instances directly from `NightBattleInput`, so `BattleState` no longer needs the field.

**Impact**: `BattleState::from_context()` already reads `context.is_sortie` and passes it to `BattleRuntimeShip::new()`. The field on `BattleState` itself is never read afterward. Removing it is safe — `cargo check` will catch any missed references.

### D3. Formation modifier deduplication

**Decision**: Replace `shelling_formation_modifier` and `torpedo_formation_modifier` with a single `formation_modifier` function. Both currently have identical implementations (same match arms, same values). Keep `asw_formation_modifier` separate because its values differ.

**Alternative considered**: Keep both functions and have one call the other. Rejected — having two names for the same function is confusing.

### D4. Documentation approach

**Decision**: Add `// NOTE:` comments at the point of simplification (kouku.rs Stage2) and a `///` doc comment on `simulate_day` explaining RNG continuity. No new modules or files needed.

## Risks / Trade-offs

- [NightBattleInput breaking change] → The struct is `pub` but only consumed by `emukc_gameplay`. One call site in `orchestrate.rs` (sortie) and one in `practice/orchestrate.rs`. Both need updating. `cargo check --workspace` catches any missed sites.
- [BattleState.is_sortie removal] → If any external consumer reads this field, compilation fails. The field was `pub(crate)` so only `emukc_battle` internals could access it. Low risk.
- [Formation modifier dedup] → Pure refactor, no behavior change. If future KanColle updates differentiate shelling/torpedo formation modifiers, the single function can be split again.

## Migration Plan

1. Add `is_sortie` to `NightBattleInput` (additive, non-breaking initially if using `..Default` but we don't derive Default, so it's breaking).
2. Update `simulate_night` to use `input.is_sortie` when constructing `BattleRuntimeShip` instances.
3. Remove `BattleState.is_sortie` field.
4. Update callers in `emukc_gameplay`.
5. Add regression test for sortie night sinking protection.
6. Documentation and dedup changes (no migration needed).

Rollback: each step is mechanical; revert in reverse order.
