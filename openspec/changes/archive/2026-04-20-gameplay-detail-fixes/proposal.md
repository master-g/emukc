## Why

Several gameplay mechanics diverge from KanColle's actual behavior, causing incorrect game state and battle outcomes. Ships can accumulate experience beyond level 99 without marriage (the level table shares the same threshold for lv99 and lv100, so unmarried ships silently reach 100+), CV opening airstrikes only hit a single target instead of multiple (real KanColle splits dive bombing and torpedo bombing into two independent sub-phases), ship remodeling writes equipment IDs into the aircraft capacity array instead of the slot array (leaving slots empty and onslot values polluted with database IDs), and some ship slot data contains implausible aircraft capacities caused by this same remodel bug. These bugs undermine gameplay fidelity.

## What Changes

- **Level 99 cap enforcement**: Ships at level 99 that have not undergone the marriage ceremony will receive 0 experience from sorties and practice battles. The `married` flag from the database will be propagated through `BattleShipInput` so that both response data and server-side processing correctly zero exp for capped ships.
- **Remodel slot/onslot fix**: In `remodel_impl`, the line `new_ship.api_onslot[i] = m.id` will be changed to `new_ship.api_slot[i] = m.id`. The current code writes equipment database IDs into `api_onslot` (aircraft capacity array) instead of `api_slot` (equipment slot array), causing remodeled ships to have empty equipment slots and garbage aircraft capacity values. This single bug is the root cause of both the remodel equipment issue and most slot capacity anomalies.
- **CV multi-target airstrike**: The `simulate_kouku` Stage 3 will be split into two sub-phases — dive bombing (艦爆) and torpedo bombing (艦攻) — each with per-slot independent target selection. Each slot with bombers independently selects a random alive target, and damage accumulates per-ship in the response arrays.
- **Existing data cleanup**: Ships that were previously remodeled will have corrupted `onslot` and `slot` values in the database. A migration or repair step will identify and fix affected ships.

## Capabilities

### New Capabilities

_None_

### Modified Capabilities

- `sortie`: Experience gain must respect level 99 cap for unmarried ships; CV airstrike multi-target behavior changes sortie battle outcomes
- `battle-damage-foundation`: Airstrike Stage 3 split into dive bombing + torpedo bombing sub-phases, each with per-slot independent targeting
- `fleet`: Remodel equipment assignment and slot capacity correctness

## Impact

- `crates/emukc_gameplay/src/game/compose/remodel.rs:175` — fix `api_onslot[i]` → `api_slot[i]`
- `crates/emukc_gameplay/src/game/battle/core.rs` — refactor `simulate_kouku` Stage 3 into dive/torpedo sub-phases; add `married` to `BattleShipInput`
- `crates/emukc_gameplay/src/game/sortie_result.rs` — add level cap check using `married` field
- `crates/emukc_gameplay/src/game/battle/practice.rs` — add level cap check using `married` field
- `crates/emukc_gameplay/src/game/sortie.rs` — populate `married` in `build_sortie_friend_ships`
- `crates/emukc_gameplay/src/game/practice.rs` — populate `married` in `build_practice_friend_ships`
- No API contract changes; all fixes are server-side behavioral corrections

## Non-goals

- Overhauling the entire battle system or damage formulas
- Adding new ship types, equipment, or game mechanics
- Changing the marriage system itself (requirements, effects, ceremony flow)
- Modifying client-side behavior or API response structure
