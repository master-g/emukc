## Why

Battle damage is clamped to target HP in `apply_damage` (`core.rs:190`), so the API response never shows overkill values. Real KanColle displays the full calculated damage even when it exceeds remaining HP (e.g., 150 damage against a 10 HP enemy shows 150, not 10). This makes battle feedback inaccurate and diverges from client expectations.

## What Changes

- `apply_damage` returns both raw (pre-clamp) and effective (HP-subtracted) damage values instead of a single `i64`
- All API response fields (`api_damage`, `api_fydam`/`api_eydam`, `api_fdam`/`api_edam`) use raw damage for display
- HP tracking continues to use effective (clamped) damage
- `damage_dealt` (MVP calculation) uses effective damage — only actual HP removed counts

## Capabilities

### New Capabilities

_None_

### Modified Capabilities

- `sortie`: Damage display in battle API responses changes from clamped to raw values. HP subtraction behavior unchanged.

## Impact

- `crates/emukc_gameplay/src/game/battle/core.rs` — `apply_damage` signature change, all ~14 call sites updated
- `crates/emukc_gameplay/src/game/battle/` packet structs — damage recording uses raw values
- Existing battle tests may need expected-value updates if they assert on damage numbers

## Non-goals

- Changing damage calculation formulas
- Changing sinking protection behavior
- Changing HP tracking or ship survival logic
