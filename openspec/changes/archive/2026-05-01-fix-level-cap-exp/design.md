## Context

Ship level in KanColle is capped at 99 unless the ship is married (ring used). Married ships can reach level 175 (or 180 with special mechanics). The `exp_to_ship_level` function converts raw XP to a level value, returning up to 180 without any marriage check. Individual XP application sites are responsible for enforcing the cap.

The sortie path (`sortie_result.rs:121-122`) correctly guards: `if !ship.married && ship.ship.api_lv >= 99 { gain = 0 }`. The practice path may lack this guard.

## Goals / Non-Goals

**Goals:**
- All XP-granting paths enforce level 99 cap for unmarried ships
- No unmarried ship can exceed level 99 through any game mechanic

**Non-Goals:**
- Changing level cap values
- Changing marriage mechanics
- Refactoring XP tables

## Decisions

### D1: Centralized level cap enforcement

**Decision**: Add a helper function `ship_level_cap(married: bool) -> i64` returning 99 or 175. Use this in all XP application paths to clamp both XP gain and resulting level.

**Alternative considered**: Enforce cap in `exp_to_ship_level` itself — rejected because the function is pure (takes only exp, no marriage context) and is used in non-ship contexts (HQ level).

### D2: Guard placement

**Decision**: Add the guard at XP **application** time (where ship level is updated), not at XP **calculation** time. This ensures the XP display values in API responses still show what would have been gained, while the actual level doesn't change.

**Rationale**: Matches sortie behavior where `get_ship_exp` returns 0 for capped ships but the battle itself is still processed normally.

## Risks / Trade-offs

- **[Risk] Missed XP paths**: Unknown XP sources may exist → Mitigate with codebase-wide search for `exp_to_ship_level` calls and `api_exp` writes
- **[Risk] Double-capping**: If both calculation and application enforce the cap, behavior is correct but wasteful → Acceptable, idempotent
