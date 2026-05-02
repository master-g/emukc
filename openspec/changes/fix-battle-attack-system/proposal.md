## Why

The battle system has three interrelated bugs in attack eligibility and damage application:

1. **Shelling display type**: Attack display type selection uses equipment lists (`DAY_SURFACE_DISPLAY_TYPES`) as the primary determinant, causing DD with only torpedo to show torpedo attack animation during shelling phase. Real game uses ship type for participation, equipment for display type selection.
2. **Closing torpedo whitelist**: `can_closing_torpedo_ship` restricts participation to a hardcoded ship type list that excludes BB with base torpedo > 0 (Bismarck drei, Гангут, etc.) and includes types that have base torpedo = 0 (DE, LHA). Per wikiwiki, the actual rule is `api_raisou[0] > 0` for any ship type.
3. **Enemy overkill**: `apply_damage` caps ALL damage to current HP, preventing overkill display against enemy ships in sortie.

## What Changes

- Fix shelling attack display type selection: ship type determines participation, equipment affects display type and damage only
- Fix closing torpedo participation: remove restrictive ship type whitelist, use `api_raisou[0] > 0` (base torpedo stat) as sole gate per wikiwiki rule
- Fix opening torpedo participation: preserve equipment-based gate (甲标的/minisub) for non-SS/CLT ships, add SS level ≥ 10 requirement
- Fix `apply_damage` to allow excess (overkill) damage against enemy ships in sortie battles (non-practice)
  - `BattleRuntimeShip` already has `is_friendly`/`is_sortie` fields — change capping logic, not signature
- Keep sinking protection (轟沈ストッパー) for friendly sortie ships unchanged
- Keep practice battle damage capped to current HP (no sinking, no excess)

## Capabilities

### New Capabilities

- `battle-attack-type`: Correct attack eligibility rules per wikiwiki. Shelling: ship type based (SS/SSV excluded, CV conditional on planes, all others always). Closing torpedo: base torpedo stat > 0 (any ship type). Opening torpedo: minisub equipment OR CLT OR SS/SSV level ≥ 10.

### Modified Capabilities

- `battle-damage-foundation`: Excess damage against enemies in sortie battles. `apply_damage` already has `is_friendly`/`is_sortie` context on `BattleRuntimeShip` — change capping logic, not signature.

## Non-goals

- Night battle attack type overhaul (separate concern, can be addressed later)
- ASW attack type changes (already ship-type-based)
- Changing the sinking protection logic for friendly ships
- Equipment improvement bonus changes (separate system)

## Impact

- `crates/emukc_gameplay/src/game/battle/core.rs` — primary file, attack type determination and damage application
- `crates/emukc_gameplay/src/game/battle/sortie.rs` — sortie battle handlers
- `crates/emukc_gameplay/src/game/battle/practice.rs` — practice battle handlers (verify no regression)
- `crates/emukc_gameplay/src/game/sortie_result.rs` — HP snapshot for battle results
- Requires wikiwiki audit of battle phase rules before implementation
