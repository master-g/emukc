## Context

Quest progress is driven by `QuestActionEvent` — an enum of 9 variants dispatched via `update_quest_progress_for_action`. Each variant maps to one or more `Kc3rdQuestCondition` types through `matches_event` → `apply_event`. Currently 4 condition types have no matching event variant or alternative handling, making affected quests uncompletable.

The event matching pipeline: gameplay action → construct `QuestActionEvent` → `update_quest_progress_for_action` iterates activated quests → `matches_event` filters → `apply_event` decrements counters → progress recalculated.

## Goals / Non-Goals

**Goals:**
- Enable progress tracking for Scrap::SpecificItems (106 conditions), Modernization (11), Sink (9), SlotItemImprovement (2)
- Maintain backward compatibility — existing triggered quests unaffected
- Wire triggers into existing gameplay call sites (powerup, sortie_result, slot_item destroy)

**Non-Goals:**
- Implementing the equipment improvement (改修工廠) gameplay system
- Changing quest reward, lifecycle, or reset logic
- Adding new KCSAPI endpoints

## Decisions

### D1: Extend SlotItemScrapped event with stars field

**Decision**: Add `stars: i64` to `QuestActionEvent::SlotItemScrapped`.

**Alternative considered**: Separate `SpecificItemScrapped` event — rejected because `SpecificItems` and `AnyEquipment` trigger at the same call site (equipment destruction). A single event simplifies the caller.

**Rationale**: `Kc3rdQuestConditionSlotItem` has a `stars` field. Matching SpecificItems requires knowing the scrapped item's star level. The `item.level` field in `SlotItem` already stores this. No Codex lookup needed at the call site.

### D2: SpecificItems matching uses Codex for equip-type resolution

**Decision**: In `apply_event`, when matching `SpecificItems`, use the Codex to resolve `item_mst_id → api_type(3)` for `EquipType` checks. Pass Codex via the existing `apply_event_with_context` method (already takes `Option<&Codex>`).

**Rationale**: `Kc3rdQuestConditionSlotItemType::EquipType(Vec<i64>)` matches by `api_mst_slotitem.api_type(3)`, which requires a Codex lookup. The `apply_event_with_context` signature already supports this.

### D3: Modernization event carries target + material ship mst IDs

**Decision**: `ModernizationCompleted { target_ship_mst_id, material_ship_mst_ids }`.

**Alternative considered**: Only pass target_ship_mst_id — rejected because `Kc3rdQuestConditionModernization` checks both `target_ship` and `material_ship` conditions, plus `batch_size` (minimum material ships per modernization).

**Rationale**: The condition struct has `target_ship: Kc3rdQuestConditionShip`, `material_ship: Kc3rdQuestConditionShip`, and `batch_size`. All three must be validated.

### D4: Sink event fires per sunk ENEMY ship in sortie result processing

**Decision**: `EnemyShipSunk { ship_stype: i64 }`, fired once per sunk enemy ship. Event carries the enemy ship's stype (api_stype), not mst_id, because all 9 Sink quest conditions match by `Kc3rdQuestConditionShip::ShipType`.

**Rationale**: Sink quests track sinking ENEMY ships (e.g., "敵空母を３隻撃沈せよ" — sink 3 enemy carriers, ShipType 11). The condition `Sink(Kc3rdQuestConditionShip, i64)` uses ShipType matching. Firing one event per sunk enemy ship lets the counter decrement naturally.

**Data source**: The `SortieBattleSession.enemy` vector contains `BattleRuntimeShip` instances with `ship.api_stype`. The session's `packet.enemy_nowhps` tracks post-battle HP. Enemy ships with `hp <= 0` are sunk.

**Call site**: `sortie.rs` (~line 567) — after `update_sortie_result_stats` and alongside the existing `SortieBattleCompleted` quest event. The `SortieBattleResultSnapshot` must be extended with `enemy_ship_types: Vec<i64>` and `enemy_nowhps: Vec<i64>` to carry this data from the battle session to the quest trigger point.

**Snapshot extension**: The battle session stores `SortieBattleSession.enemy` with full ship data, but the current snapshot only carries `enemy_ship_ids` (mst_ids). Add the two fields to `SortieBattleResultSnapshot` and populate them from the session when creating the snapshot (in `sortie.rs` where snapshots are built).

### D5: SlotItemImproved as hook-only event

**Decision**: Add the event variant and matching logic. No gameplay caller.

**Rationale**: 2 quests use `SlotItemImprovement` but the improvement system isn't implemented. Adding the event now ensures the quest condition type is properly handled when improvement is built later. Zero cost — unused enum variants compile away.

## Risks / Trade-offs

**SlotItemScrapped stars field is a breaking change to the event enum** → All existing callers must pass `stars: 0` or the actual item level. Two call sites: `slot_item.rs:444` and `factory.rs` (ship scrap with equipment). Both have access to item level data.

**Sink trigger fires inside sortie result transaction** → EnemyShipSunk events fire alongside the existing SortieBattleCompleted event in the same transaction. If quest update fails, the entire sortie result rolls back. This is acceptable — quest progress is secondary to battle result correctness. The existing `SortieBattleCompleted` event already follows this pattern.

**SortieBattleResultSnapshot extension is required** → Adding `enemy_ship_types` and `enemy_nowhps` fields changes the snapshot struct used in tests and battle processing. All snapshot construction sites (day battle, night battle, SP midnight) must be updated to populate the new fields from the battle session.

**SpecificItems matching performance** → For each scrapped item, iterates all activated quests with SpecificItems conditions, then iterates the specific items list. With typically <20 active quests, this is negligible.

**SlotItemImproved has no tests possible** → The event/matching can be unit-tested, but integration testing requires the improvement system. Acceptable for a hook.
