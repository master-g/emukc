## Why

Two independent ship stat bugs: (1) Ship remodel does not restore HP to the new maximum — the ship retains its pre-remodel HP while max HP increases, leaving it damaged. (2) Repair dock time calculation does not account for the CT (練習巡洋艦) ship type's correct modifier, and lacks the "instructor ship" repair time reduction mechanic present in real KanColle.

## What Changes

- Set `api_nowhp = api_maxhp` after ship remodel completes
- Verify/fix CT ship type modifier in repair time calculation
- Investigate whether the "fleet has CT → reduced dock time" mechanic needs implementation

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

_(none — bug fixes within existing capabilities)_

## Non-goals

- Changing remodel material costs or requirements
- Changing the repair time formula structure
- Implementing other docking features (instant repair item usage, etc.)

## Impact

- `crates/emukc_gameplay/src/game/compose/remodel.rs` — HP restoration after remodel
- `crates/emukc_gameplay/src/game/ndock.rs` — CT modifier / instructor mechanic
- `crates/emukc_model/src/codex/repair.rs` — ship_type_mod table
