## Why

Taiha (≤25% HP) torpedo attacks multiply power by 0.0, making `capped_power = 0`. The `resolve_damage` function then falls into the scratch-damage branch (`0 < defense`), dealing proportional damage instead of zero. Taiha ships should not deal any torpedo damage.

## What Changes

- Add a zero-power guard in `resolve_damage`: when `capped_power ≤ 0`, return 0 immediately instead of triggering scratch damage.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `battle-damage-foundation`: `resolve_damage` must handle zero/negative capped power by returning 0 damage (no scratch).

## Impact

- Single function: `resolve_damage` in `crates/emukc_gameplay/src/game/battle/core.rs`
- Affects only the taiha torpedo edge case; all other damage flows unchanged
- One new test needed to verify taiha torpedo deals 0 damage

## Non-goals

- Attack eligibility (skipping the attack entirely) is caller responsibility, not `resolve_damage`'s concern
- No changes to damage state thresholds or other damage formulas
