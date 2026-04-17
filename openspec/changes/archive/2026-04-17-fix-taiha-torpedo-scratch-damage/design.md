## Context

`resolve_damage()` in `crates/emukc_gameplay/src/game/battle/core.rs` decides between scratch and normal damage. It checks `capped_power < defense` — but when `capped_power` is 0 (taiha torpedo, `damage_state_modifier` returns 0.0), this always triggers scratch damage. The correct behavior is 0 damage.

## Goals / Non-Goals

**Goals:**
- Return 0 damage when `capped_power ≤ 0` before scratch-damage logic runs

**Non-Goals:**
- Skipping the attack entirely (caller's responsibility)
- Changing damage state thresholds or other formulas

## Decisions

**Guard at top of `resolve_damage`**: Add `if capped_power <= 0.0 { return 0; }` as the first check. This is minimal, correct, and doesn't affect any other path.

Alternative considered: check at each call site — rejected because `resolve_damage` is the single choke point and centralizing the guard is simpler.

## Risks / Trade-offs

None. The guard only fires when power is exactly 0 or negative (which can't happen in normal flow). All existing tests pass with this change.
