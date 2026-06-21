---
title: "Quest lifecycle, cross-cutting progress tracking, and rewards"
date: 2026-06-22
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Implementing or modifying quest activation, progress evaluation, or reward distribution"
  - "Adding a new gameplay action that should advance quest progress"
  - "Authoring or consuming third-party quest manifest data (Kc3rdQuestMap)"
tags: [quest, progress-tracking, rewards, conditions, cross-cutting, questops]
related_components: [emukc_model, emukc_db]
---

# Quest lifecycle, cross-cutting progress tracking, and rewards

## Context

Quests follow a state machine and receive progress updates from virtually
every gameplay domain. `QuestOps` (`emukc_gameplay`) manages activation,
condition evaluation (And/OneOf/Sequential), reward distribution, and
persistence across three entity types under `entity::profile::quest`.
Definitions come from the Codex's third-party quest data (`Kc3rdQuestMap`).
Migrated from `openspec/specs/quest/spec.md`.

## Guidance

### Quest lifecycle

Quests SHALL follow a state machine: idle → activated → completed (claimed),
via `QuestOps`. Definitions come from the Codex `Kc3rdQuestMap`.

- Availability: when quest records are retrieved (`get_quest_records`), the
  quest tree is updated via `update_quests_impl` (adds new quests, removes
  expired); composition quests are validated against the current fleet state;
  quests are filtered by period (daily/weekly/monthly/etc.) and reset timers;
  prerequisite quest chains are respected (quest X may require quest Y to be
  completed first).
- Activation (`quest_start`) on an idle quest: status → Activated; if the quest
  has Exercise conditions, activation headroom may be applied (sets `times=1`
  if inactive had `times<=0`); if already at Completed progress, it is reset to
  Eighty to require one real action; composition quests are re-validated after
  activation.
- Deactivation (`quest_stop`) on an activated quest: status → Idle; progress is
  preserved (not reset).
- Calling start/stop on a quest already in the target status SHALL fail with a
  `QuestStatusInvalid` error.
- Completion (`quest_clear_and_claim_reward`) on a quest with Activated status
  and Completed progress: the quest is marked completed (oneshot or periodic
  record); the progress record is deleted; the quest tree is reconstructed via
  `update_quests_impl`; consumption requirements are deducted (materials,
  items); rewards are granted per the definition.

### Quest progress tracking (cross-cutting)

Quest progress SHALL be updated by gameplay actions across all domains via
`update_quest_progress_for_action` (public) and `update_quests_impl`
(internal).

- When a qualifying gameplay action occurs (sortie win, composition change,
  equipment improvement, construction complete, etc.), all activated quests
  matching the action type have their progress evaluated and updated.
- Condition types: `And` requires all sub-conditions met; `OneOf` requires at
  least one met; `Sequential` requires sub-conditions met in order.
- Composition validation (`validate_composition_quests`, typically after fleet
  changes) checks composition conditions against current fleet state and
  updates progress.
- Scrap specific items: when equipment is scrapped via `destroy_items`, the
  `SlotItemScrapped` event includes the item's star level; `Scrap::SpecificItems`
  conditions match on item `mst_id` (Equipment type), equip type via Codex
  lookup (EquipType type), and minimum star level; the matched item's amount
  counter is decremented; `SpecificItems` is satisfied when all contained items
  have `amount == 0`.
- Modernization: on successful modernization (powerup), the
  `ModernizationCompleted` event is fired with target ship `mst_id` and
  material ship `mst_ids`; `Modernization` conditions check `target_ship`
  against target's ship type/class/id and `material_ship` against all material
  ships; if material ships count meets `batch_size`, the times counter is
  decremented.
- Enemy ship sinking: when an enemy ship is sunk (enemy HP ≤ 0 after battle),
  the `SortieBattleResultSnapshot` SHALL include `enemy_ship_types` and
  `enemy_nowhps` populated from the battle session; the `EnemyShipSunk` event
  is fired with each sunk enemy's `stype` (`api_stype`); `Sink` conditions
  check `Kc3rdQuestConditionShip` (ShipType matching) and decrement the count
  on match.
- Slot item improvement (改修工廠, future feature): on completion, the
  `SlotItemImproved` event is fired with item `mst_id` and resulting star
  level; `Factory::SlotItemImprovement` conditions decrement the count.

### Quest rewards

Rewards SHALL be defined in the Codex (`Kc3rdQuest`) and can include
materials, slot items, ships, use items, fleet unlocks, and large construction
unlocks.

- Basic material rewards (fuel/ammo/steel/bauxite) are added via
  `add_material_impl`, capped at the profile maximum.
- Special material rewards (torch/bucket/devmat/screw) are added via
  `add_material_impl` with the appropriate `MaterialCategory`.
- Slot item rewards are added via `add_slot_item_impl` with specified
  improvement stars.
- Ship rewards are added via `add_ship_impl`.
- Fleet unlock rewards call `unlock_fleet_impl` for the specified fleet index.
- Large construction unlock rewards call `unlock_large_construction_impl`.
- `choice_rewards`: the player selects one option from each choice group via
  `reward_choices`; only selected rewards are granted.

### Data sources and persistence

- Quest metadata is looked up from `codex.quest` by quest ID via
  `Kc3rdQuest::find_in_codex`; a missing ID fails with an appropriate error.
- Progress records store: `quest_id`, status (Idle/Activated), progress
  (percentage/Completed), period, `start_since`, requirements (serialized
  condition tree), `requirement_type` (And/OneOf/Sequential).
- Oneshot completion creates a record in `quest::oneshot` to prevent
  re-appearance.
- Periodic completion creates a record in `quest::periodic` with the
  completion timestamp; the quest re-appears in the next period after reset.

## Why This Matters

Quest progress is the most cross-cutting subsystem — nearly every gameplay
action can advance it. The event-firing + condition-evaluation contract is
what lets domains stay decoupled while still advancing quests. Reward routing
reuses the `_impl` material/ship/slot helpers so rewards participate in
transactions and respect caps.

## When to Apply

- When adding a new gameplay action that should advance quests (add a matching
  event type and condition matcher).
- When modifying reward distribution or the quest state machine.
- When consuming new quest definitions from the third-party manifest.

## Examples

- A sortie win fires `EnemyShipSunk` events; `Sink` quests matching the sunk
  stype decrement.
- `Scrap::SpecificItems` with a ★min-star requirement matches only scrapped
  equipment at or above that star level.
- `choice_rewards` grants only the player-selected option per choice group.

## Related

- `docs/solutions/architecture-patterns/material.md` — rewards route through
  `add_material_impl` and respect caps.
- `docs/solutions/architecture-patterns/fleet.md` — `unlock_fleet_impl`.
