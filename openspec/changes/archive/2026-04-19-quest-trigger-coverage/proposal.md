## Why

Quest system has 4 condition types with no trigger mechanism, leaving ~128 quest conditions unable to progress. Scrap::SpecificItems (106 conditions) is the largest gap — these quests require scrapping specific equipment but `matches_event` never matches them. Modernization (11), Sink (9), and SlotItemImprovement (2) lack `QuestActionEvent` variants entirely.

## What Changes

- Extend `QuestActionEvent::SlotItemScrapped` with `stars` field to support `Scrap::SpecificItems` matching by equipment ID, type, and star level
- Add `QuestActionEvent::ModernizationCompleted` variant with target/material ship info
- Add `QuestActionEvent::EnemyShipSunk` variant with enemy ship stype (ship type) — Sink quests track enemy ships sunk, not friendly ship losses
- Add `QuestActionEvent::SlotItemImproved` variant (hook only — improvement system not yet implemented)
- Extend `matches_event` and `apply_event` in `matcher.rs` for all new condition-event pairs
- Fix `is_satisfied` for `Scrap::SpecificItems` (currently always `false`)
- Wire quest triggers into `compose/powerup.rs` (modernization), `sortie.rs` (enemy ship sinking), and `slot_item.rs`/`factory.rs` (stars field)
- Extend `SortieBattleResultSnapshot` with `enemy_ship_types` and `enemy_nowhps` fields to carry enemy ship data for Sink quest evaluation

## Capabilities

### New Capabilities

(none — all changes are within the existing quest system)

### Modified Capabilities

- `quest`: Quest Progress Tracking requirement changes — new event types, new condition matching for SpecificItems/Modernization/Sink/SlotItemImprovement

## Non-goals

- Implementing the equipment improvement (改修工廠) gameplay system — only the quest event hook
- Changing quest reward or lifecycle logic
- Adding new quest definitions to the codex

## Impact

- `crates/emukc_model/src/thirdparty/quest/matcher.rs` — core event matching logic
- `crates/emukc_model/src/thirdparty/quest/progress.rs` — `is_satisfied` for SpecificItems
- `crates/emukc_gameplay/src/game/slot_item.rs` — SlotItemScrapped event data
- `crates/emukc_gameplay/src/game/factory.rs` — SlotItemScrapped event data (ship scrap with equipment)
- `crates/emukc_gameplay/src/game/compose/powerup.rs` — new quest trigger call
- `crates/emukc_gameplay/src/game/sortie.rs` — populate enemy data in snapshot, fire EnemyShipSunk events per sunk enemy ship
- `crates/emukc_gameplay/src/game/sortie_result.rs` — extend `SortieBattleResultSnapshot` with enemy fields
