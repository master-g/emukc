## Why

Current battle damage formulas in `emukc_gameplay` use simplified approximations that diverge significantly from the real KanColle formulas. Defense is a fixed multiplier instead of randomized, damaged ships fight at full power, improvement bonuses from ★ equipment are ignored, and scratch damage logic is wrong. These gaps make battle outcomes both predictable and inaccurate. Phase 1 fixes the foundation: defense randomization, damage state, scratch triggers, and torpedo base power.

## What Changes

- Replace fixed armor multiplier (`A×0.7` / `A×0.55` / `A×0.6`) with randomized defense formula `floor(0.7×A + 0.6×rand(0, floor(A)−1))` for all attack types
- Add damage state modifier (chuuha ×0.7/0.8, taiha ×0.4/0) as pre-cap multiplier for shelling, torpedo, and ASW
- Fix scratch damage trigger: when capped attack power < defense, use `floor(0.06×H + 0.08×rand(0, H−1))` instead of forcing minimum 1
- Fix torpedo basic power: remove `+5` constant (torpedo uses `TP + improvement_bonus`, not `TP + 5`)
- Add `calculate_defense_power()` as shared function used by all damage calculators

## Capabilities

### New Capabilities

- `battle-damage-foundation`: Correct defense randomization, damage state, scratch damage, and torpedo base power for all attack types

### Modified Capabilities

None. This changes internal battle calculation behavior without changing external API contracts.

## Impact

- **Code**: `crates/emukc_gameplay/src/game/battle/core.rs` — all `calculate_*_damage()` functions, `simulate_shelling_side()`, `simulate_raigeki()`, `simulate_opening_torpedo()`
- **Behavior**: Battle outcomes change immediately — damage becomes less predictable (defense randomization), damaged ships deal less damage, scratch damage occurs correctly, torpedo power slightly decreases
- **Tests**: Existing battle tests may need threshold adjustments due to randomized defense
- **No API changes**: Response format unchanged

## Non-goals

- Improvement bonus (改修強化) — requires star-level equipment data, deferred to phase 2
- CV special shelling formula — deferred to phase 2
- Critical hits, artillery spotting, AP shell — deferred to phase 3
- Night battle recon bonus, carrier night attack — deferred to phase 5
- ASW armor penetration — deferred to phase 6
