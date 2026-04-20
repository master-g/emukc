## Purpose
Quest lifecycle and cross-cutting progress tracking for EmuKC. Covers quest
activation/completion, condition evaluation (And/OneOf/Sequential), reward
distribution, and the quest progress update mechanism triggered by all domains.

## Requirements

### Requirement: Quest Lifecycle
Quests SHALL follow a state machine: idle -> activated -> completed (claimed).
Quest definitions MUST come from the Codex's third-party quest data (Kc3rdQuestMap).
Implemented via QuestOps.

#### Scenario: Quest availability
- WHEN quest records are retrieved for a profile via get_quest_records
- THEN the quest tree is updated via update_quests_impl (adds new quests, removes expired)
- THEN composition quests are validated against the current fleet state
- THEN quests are filtered based on their period (daily, weekly, monthly, etc.) and reset timers
- THEN prerequisite quest chains are respected (quest X may require quest Y to be completed first)

#### Scenario: Quest activation (start)
- WHEN quest_start is called for an idle quest
- THEN the quest status changes to Activated
- THEN if the quest has Exercise conditions, activation headroom may be applied (sets times=1 if inactive had times<=0)
- THEN if the quest was already at Completed progress, it is reset to Eighty to require one real action
- THEN composition quests are re-validated after activation

#### Scenario: Quest deactivation (stop)
- WHEN quest_stop is called for an activated quest
- THEN the quest status returns to Idle
- THEN progress is preserved (not reset)

#### Scenario: Quest status already at target
- WHEN quest_start or quest_stop is called for a quest already in the target status
- THEN the operation fails with a QuestStatusInvalid error

#### Scenario: Quest completion and reward claim
- WHEN quest_clear_and_claim_reward is called for a quest with Activated status and Completed progress
- THEN the quest is marked as completed (oneshot or periodic record)
- THEN the quest progress record is deleted
- THEN the quest tree is reconstructed via update_quests_impl
- THEN consumption requirements are deducted (materials, items)
- THEN rewards are granted based on quest definition

### Requirement: Quest Progress Tracking
Quest progress SHALL be updated by gameplay actions across all domains via
update_quest_progress_for_action (public) and update_quests_impl (internal).
This is the primary cross-cutting mechanism: virtually every gameplay action
MUST be able to trigger quest progress updates.

#### Scenario: Progress update on gameplay action
- WHEN a qualifying gameplay action occurs (sortie win, composition change, equipment improvement, construction complete, etc.)
- THEN all activated quests matching the action type have their progress evaluated
- THEN progress is updated based on the quest's condition evaluation logic

#### Scenario: Quest condition types
- WHEN quest conditions are evaluated
- THEN And conditions require all sub-conditions to be met
- THEN OneOf conditions require at least one sub-condition to be met
- THEN Sequential conditions require sub-conditions to be met in order

#### Scenario: Composition quest validation
- WHEN validate_composition_quests is called (typically after fleet changes)
- THEN quests with composition conditions are checked against current fleet state
- THEN progress is updated if the fleet now meets (or no longer meets) the condition

#### Scenario: Scrap specific items progress
- WHEN equipment is scrapped via destroy_items
- THEN the SlotItemScrapped event includes the item's star level (improvement level)
- THEN quests with Scrap::SpecificItems conditions are matched against the scrapped item
- THEN matching considers item mst_id (for Equipment type), equip type via Codex lookup (for EquipType type), and minimum star level
- THEN the matching item's amount counter is decremented
- THEN SpecificItems is satisfied when all contained items have amount == 0

#### Scenario: Modernization progress
- WHEN ship modernization (powerup) completes successfully
- THEN the ModernizationCompleted event is fired with target ship mst_id and material ship mst_ids
- THEN quests with Modernization conditions check target_ship against the target's ship type/class/id
- THEN material_ship conditions are checked against all material ships
- THEN if material ships count meets batch_size, the times counter is decremented

#### Scenario: Enemy ship sinking progress
- WHEN an enemy ship is sunk during sortie (enemy HP <= 0 after battle)
- THEN the SortieBattleResultSnapshot SHALL include enemy_ship_types and enemy_nowhps populated from the battle session
- THEN the EnemyShipSunk event is fired with the sunk enemy ship's stype (api_stype) for each enemy ship with HP <= 0
- THEN quests with Sink conditions check the Kc3rdQuestConditionShip (ShipType matching) against the sunk enemy ship's stype
- THEN the sink count is decremented on match

#### Scenario: Slot item improvement progress
- WHEN equipment improvement completes (改修工廠 — future feature)
- THEN the SlotItemImproved event is fired with item mst_id and resulting star level
- THEN quests with Factory::SlotItemImprovement conditions decrement the count

### Requirement: Quest Rewards
Quest rewards SHALL be defined in the Codex (Kc3rdQuest) and can include materials,
slot items, ships, use items, fleet unlocks, and large construction unlocks.

#### Scenario: Material rewards
- WHEN a quest grants basic material rewards (fuel, ammo, steel, bauxite)
- THEN materials are added to the profile via add_material_impl
- THEN materials are capped at the profile's maximum (see material capability)

#### Scenario: Special material rewards
- WHEN a quest grants special material rewards (torch, bucket, devmat, screw)
- THEN materials are added via add_material_impl with the appropriate MaterialCategory

#### Scenario: Slot item rewards
- WHEN a quest grants slot item rewards
- THEN the item is added to the profile via add_slot_item_impl with specified improvement stars

#### Scenario: Ship rewards
- WHEN a quest grants ship rewards
- THEN the ship is added to the profile via add_ship_impl

#### Scenario: Fleet unlock rewards
- WHEN a quest grants a fleet unlock reward
- THEN unlock_fleet_impl is called for the specified fleet index

#### Scenario: Large construction unlock rewards
- WHEN a quest grants a large ship construction unlock
- THEN unlock_large_construction_impl is called

#### Scenario: Multiple reward choices
- WHEN a quest has choice_rewards
- THEN the player selects one option from each choice group via reward_choices parameter
- THEN only the selected rewards are granted

### Requirement: Quest Data Sources
Quest definitions SHALL come from the Codex's third-party quest data (Kc3rdQuestMap).

#### Scenario: Quest definition lookup
- WHEN quest metadata is needed (conditions, rewards, period)
- THEN the quest manifest is looked up from codex.quest by quest ID via Kc3rdQuest::find_in_codex

#### Scenario: Missing quest definition
- WHEN a quest ID is not found in the Codex
- THEN the lookup fails with an appropriate error

### Requirement: Quest Progress Persistence
Quest state SHALL be persisted across three entity types under entity::profile::quest.

#### Scenario: Quest progress record
- WHEN a quest progress record is stored
- THEN it contains: quest_id, status (Idle/Activated), progress (percentage/Completed), period, start_since timestamp, requirements (serialized condition tree), requirement_type (And/OneOf/Sequential)

#### Scenario: Quest oneshot records
- WHEN a one-time quest is completed
- THEN a record is created in quest::oneshot to prevent re-appearance

#### Scenario: Quest periodic records
- WHEN a periodic quest (daily/weekly/monthly) is completed
- THEN a record is created in quest::periodic with the completion timestamp
- THEN the quest will re-appear in the next period after reset
