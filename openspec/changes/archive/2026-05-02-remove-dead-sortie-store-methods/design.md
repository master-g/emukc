## Context

Phase 5 of `battle-architecture-crate-extract` replaced `SortieStore`'s closure-based mutation methods with get-modify-insert pattern across all call sites in `battle/sortie/orchestrate.rs` and `game/sortie.rs`. The old methods — `modify_active_sortie`, `with_pending_result_mut`, `with_pending_battle_mut` — are now dead code with zero callers.

Additionally, `sortie/mod.rs` uses blanket `#![allow(dead_code)]` which suppresses all unused-item warnings in the module. Only `EngagementType` import is unused; the blanket suppression hides future issues.

## Goals / Non-Goals

**Goals:**
- Delete 3 dead methods from `SortieStore`
- Replace blanket `#![allow(dead_code)]` with targeted `#[allow(dead_code)]`

**Non-Goals:**
- No behavior changes
- No API changes
- No other cleanup

## Decisions

### D1: Direct deletion, no deprecation

**Decision**: Delete the methods immediately. They are `pub(super)` so only accessible within `game` module. All callers already migrated. No deprecation period needed.

**Rationale**: The compiler already warns they're unused. No external consumers exist. The get-modify-insert replacements have been in place since Phase 5 and tested via integration tests.

### D2: Keep `allow(dead_code)` on `EngagementType` import

**Decision**: After narrowing, `EngagementType` in `sortie/mod.rs` is the only unused import. It's used by sub-modules (`orchestrate.rs`) but imported in the parent. Apply `#[allow(dead_code)]` to just that import line instead of the whole module.

**Rationale**: Moving the import to the submodule that uses it would be cleaner but is a larger diff with potential knock-on effects. The targeted `allow` is minimal and reversible.
