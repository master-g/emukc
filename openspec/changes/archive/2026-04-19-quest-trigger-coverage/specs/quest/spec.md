## MODIFIED Requirements

### Requirement: Quest Progress Tracking
Quest progress SHALL be updated by gameplay actions across all domains via
update_quest_progress_for_action (public) and update_quests_impl (internal).
This is the primary cross-cutting mechanism: virtually every gameplay action
MUST be able to trigger quest progress updates.

#### Scenario: Progress update on gameplay action
- **WHEN** a qualifying gameplay action occurs (sortie win, composition change, equipment improvement, construction complete, etc.)
- **THEN** all activated quests matching the action type have their progress evaluated
- **THEN** progress is updated based on the quest's condition evaluation logic

#### Scenario: Quest condition types
- **WHEN** quest conditions are evaluated
- **THEN** And conditions require all sub-conditions to be met
- **THEN** OneOf conditions require at least one sub-condition to be met
- **THEN** Sequential conditions require sub-conditions to be met in order

#### Scenario: Composition quest validation
- **WHEN** validate_composition_quests is called (typically after fleet changes)
- **THEN** quests with composition conditions are checked against current fleet state
- **THEN** progress is updated if the fleet now meets (or no longer meets) the condition

#### Scenario: Scrap specific items progress
- **WHEN** equipment is scrapped via destroy_items
- **THEN** the SlotItemScrapped event includes the item's star level (improvement level)
- **THEN** quests with Scrap::SpecificItems conditions are matched against the scrapped item
- **THEN** matching considers item mst_id (for Equipment type), equip type via Codex lookup (for EquipType type), and minimum star level
- **THEN** the matching item's amount counter is decremented
- **THEN** SpecificItems is satisfied when all contained items have amount == 0

#### Scenario: Modernization progress
- **WHEN** ship modernization (powerup) completes successfully
- **THEN** the ModernizationCompleted event is fired with target ship mst_id and material ship mst_ids
- **THEN** quests with Modernization conditions check target_ship against the target's ship type/class/id
- **THEN** material_ship conditions are checked against all material ships
- **THEN** if material ships count meets batch_size, the times counter is decremented

#### Scenario: Enemy ship sinking progress
- **WHEN** an enemy ship is sunk during sortie (enemy HP <= 0 after battle)
- **THEN** the SortieBattleResultSnapshot SHALL include enemy_ship_types and enemy_nowhps populated from the battle session
- **THEN** the EnemyShipSunk event is fired with the sunk enemy ship's stype (api_stype) for each enemy ship with HP <= 0
- **THEN** quests with Sink conditions check the Kc3rdQuestConditionShip (ShipType matching) against the sunk enemy ship's stype
- **THEN** the sink count is decremented on match

#### Scenario: Slot item improvement progress
- **WHEN** equipment improvement completes (改修工廠 — future feature)
- **THEN** the SlotItemImproved event is fired with item mst_id and resulting star level
- **THEN** quests with Factory::SlotItemImprovement conditions decrement the count
