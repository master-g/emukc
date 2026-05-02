## Why

Ships without marriage can exceed level 99 through experience gain. The sortie battle result handler correctly checks `!married && api_lv >= 99` to block XP, but other XP-granting paths (practice battles, exercises, quests) may lack this guard. Additionally, `exp_to_ship_level` in level.rs has no marriage awareness and returns levels up to 180 unconditionally.

## What Changes

- Audit all XP-granting code paths to ensure every path enforces the level 99 cap for unmarried ships
- Ensure `exp_to_ship_level` or callers properly clamp ship level to 99 when unmarried

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

_(none — this is a bug fix within existing capabilities, no spec-level behavior change)_

## Non-goals

- Changing the level cap values (99/175)
- Changing marriage mechanics
- Refactoring the XP table structure

## Impact

- `crates/emukc_gameplay/src/game/battle/practice.rs` — practice XP application
- `crates/emukc_gameplay/src/game/sortie_result.rs` — verify sortie XP guard
- Any other XP-granting gameplay functions (quest rewards, exercises, etc.)
- `crates/emukc_model/src/kc2/level.rs` — may need level clamping in exp_to_ship_level
