//! Quest condition matching

use crate::codex::Codex;
use crate::kc2::{KcSortieResult, KcSortieResultRank};
use crate::thirdparty::{
    FleetShipSnapshot, Kc3rdQuestCondition, Kc3rdQuestConditionComposition,
    Kc3rdQuestConditionExercise, Kc3rdQuestConditionFactory, Kc3rdQuestConditionMapInfo,
    Kc3rdQuestConditionScrap, Kc3rdQuestConditionShip, Kc3rdQuestConditionSlotItemType,
    Kc3rdQuestConditionSortie, Kc3rdQuestConditionSortieMap, validate_composition_snapshot,
};

/// Quest action events
#[derive(Debug, Clone)]
pub enum QuestActionEvent {
    ShipConstructed {
        ship_mst_id: i64,
        large: bool,
    },
    SlotItemConstructed {
        item_mst_id: i64,
    },
    ShipScrapped {
        ship_mst_id: i64,
    },
    SlotItemScrapped {
        item_mst_id: i64,
        stars: i64,
    },
    ShipRepaired {
        ship_id: i64,
    },
    ShipResupplied {
        ship_id: i64,
    },
    ExpeditionCompleted {
        mission_id: i64,
        result: ExpeditionResult,
        fleet_id: i64,
    },
    ExerciseBattleCompleted {
        fleet_id: i64,
        win_rank: KcSortieResultRank,
        fleet_ships: Vec<FleetShipSnapshot>,
    },
    SortieStarted {
        maparea_id: i64,
        mapinfo_no: i64,
        fleet_id: i64,
    },
    SortieBattleCompleted {
        maparea_id: i64,
        mapinfo_no: i64,
        /// The map's gauge (1-based) when the battle was fought.
        gauge_phase: i64,
        boss_cell: bool,
        /// The cell's `node_label`.
        node: Option<String>,
        win_rank: KcSortieResultRank,
        fleet_id: i64,
        /// 第1艦隊 as it fought.
        fleet_ships: Vec<FleetShipSnapshot>,
    },
    /// A sortie fleet reached a cell where it does not fight, such as 1-6's goal.
    SortieNodeReached {
        maparea_id: i64,
        mapinfo_no: i64,
        /// The map's gauge (1-based) when the cell was reached.
        gauge_phase: i64,
        /// The cell's `node_label`.
        node: String,
        fleet_id: i64,
        /// 第1艦隊 as it arrived.
        fleet_ships: Vec<FleetShipSnapshot>,
    },
    ModernizationCompleted {
        target_ship_mst_id: i64,
        material_ship_mst_ids: Vec<i64>,
    },
    EnemyShipSunk {
        ship_stype: i64,
    },
    SlotItemImproved {
        item_mst_id: i64,
        stars: i64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpeditionResult {
    Failure = 0,
    Success = 1,
    GreatSuccess = 2,
}

impl Kc3rdQuestCondition {
    pub fn matches_event(&self, event: &QuestActionEvent) -> bool {
        matches!(
            (self, event),
            (
                Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::ShipConstruction(_)),
                QuestActionEvent::ShipConstructed { .. },
            ) | (
                Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemConstruction(_)),
                QuestActionEvent::SlotItemConstructed { .. },
            ) | (
                Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyShip(_)),
                QuestActionEvent::ShipScrapped { .. },
            ) | (
                Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::AnyEquipment(_)),
                QuestActionEvent::SlotItemScrapped { .. },
            ) | (
                Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(_)),
                QuestActionEvent::SlotItemScrapped { .. },
            ) | (
                Kc3rdQuestCondition::Modernization(_),
                QuestActionEvent::ModernizationCompleted { .. }
            ) | (Kc3rdQuestCondition::Sink(_, _), QuestActionEvent::EnemyShipSunk { .. })
                | (
                    Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(
                        _
                    )),
                    QuestActionEvent::SlotItemImproved { .. },
                )
                | (Kc3rdQuestCondition::Repair(_), QuestActionEvent::ShipRepaired { .. },)
                | (Kc3rdQuestCondition::Resupply(_), QuestActionEvent::ShipResupplied { .. },)
                | (
                    Kc3rdQuestCondition::Exercise(_),
                    QuestActionEvent::ExerciseBattleCompleted { .. },
                )
                | (
                    Kc3rdQuestCondition::Expedition(_),
                    QuestActionEvent::ExpeditionCompleted { .. },
                )
                | (Kc3rdQuestCondition::Sortie(_), QuestActionEvent::SortieBattleCompleted { .. },)
                | (Kc3rdQuestCondition::Sortie(_), QuestActionEvent::SortieNodeReached { .. },)
                // Which of the two a sortie condition counts is decided by its master.
                | (Kc3rdQuestCondition::Sortie(_), QuestActionEvent::SortieStarted { .. },)
        )
    }

    pub fn apply_event(&mut self, event: &QuestActionEvent) -> bool {
        self.apply_event_with_context(event, None, None)
    }

    pub fn apply_event_with_reference(
        &mut self,
        event: &QuestActionEvent,
        reference: Option<&Kc3rdQuestCondition>,
    ) -> bool {
        self.apply_event_with_context(event, reference, None)
    }

    pub fn apply_event_with_context(
        &mut self,
        event: &QuestActionEvent,
        reference: Option<&Kc3rdQuestCondition>,
        codex: Option<&Codex>,
    ) -> bool {
        if !self.matches_event(event) {
            return false;
        }

        match self {
            Kc3rdQuestCondition::Factory(
                Kc3rdQuestConditionFactory::ShipConstruction(count)
                | Kc3rdQuestConditionFactory::SlotItemConstruction(count)
                | Kc3rdQuestConditionFactory::SlotItemImprovement(count),
            )
            | Kc3rdQuestCondition::Scrap(
                Kc3rdQuestConditionScrap::AnyShip(count)
                | Kc3rdQuestConditionScrap::AnyEquipment(count),
            )
            | Kc3rdQuestCondition::Repair(count)
            | Kc3rdQuestCondition::Resupply(count) => {
                *count > 0 && {
                    *count -= 1;
                    true
                }
            }
            Kc3rdQuestCondition::Expedition(conditions) => {
                let QuestActionEvent::ExpeditionCompleted {
                    mission_id,
                    ..
                } = event
                else {
                    return false;
                };
                // Lists name expeditions by id ("37") or by code ("A1" is 100).
                let code = codex
                    .and_then(|codex| codex.expedition_conditions.get(mission_id))
                    .map(|expedition| expedition.code.as_str());

                for condition in conditions.iter_mut() {
                    let matches = condition.list.as_ref().is_none_or(|allowed_ids| {
                        let mission_id_str = mission_id.to_string();
                        allowed_ids.iter().any(|id| {
                            id == &mission_id_str
                                || id.parse::<i64>().ok().is_some_and(|v| v == *mission_id)
                                || code == Some(id.as_str())
                        })
                    });

                    if matches && condition.times > 0 {
                        condition.times -= 1;
                        return true;
                    }
                }

                false
            }
            Kc3rdQuestCondition::Exercise(condition) => {
                let QuestActionEvent::ExerciseBattleCompleted {
                    fleet_id,
                    win_rank,
                    fleet_ships,
                } = event
                else {
                    return false;
                };

                apply_exercise_event(condition, *fleet_id, *win_rank, fleet_ships, codex)
            }
            Kc3rdQuestCondition::Sortie(condition) => {
                let master_condition = reference.and_then(|condition| match condition {
                    Kc3rdQuestCondition::Sortie(sortie) => Some(sortie),
                    _ => None,
                });
                apply_sortie_event(condition, event, master_condition)
            }
            Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(items)) => {
                let QuestActionEvent::SlotItemScrapped {
                    item_mst_id,
                    stars,
                } = event
                else {
                    return false;
                };
                apply_specific_items_event(items, *item_mst_id, *stars, codex)
            }
            Kc3rdQuestCondition::Modernization(condition) => {
                let QuestActionEvent::ModernizationCompleted {
                    target_ship_mst_id,
                    material_ship_mst_ids,
                } = event
                else {
                    return false;
                };
                apply_modernization_event(
                    condition,
                    codex,
                    *target_ship_mst_id,
                    material_ship_mst_ids,
                )
            }
            Kc3rdQuestCondition::Sink(ship_cond, count) => {
                let QuestActionEvent::EnemyShipSunk {
                    ship_stype,
                } = event
                else {
                    return false;
                };
                if *count > 0 && ship_matches_stype(ship_cond, codex, *ship_stype) {
                    *count -= 1;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// Apply `event` to each of a quest's conditions, read against the master's
/// at the same index. A sortie condition bound to a composition only counts
/// battles fought in that composition.
pub fn apply_event_to_conditions(
    conditions: &mut [Kc3rdQuestCondition],
    master: Option<&[Kc3rdQuestCondition]>,
    event: &QuestActionEvent,
    codex: Option<&Codex>,
) -> bool {
    let mut updated = false;
    for (idx, condition) in conditions.iter_mut().enumerate() {
        if let QuestActionEvent::SortieBattleCompleted {
            fleet_id,
            fleet_ships,
            ..
        }
        | QuestActionEvent::SortieNodeReached {
            fleet_id,
            fleet_ships,
            ..
        } = event
            && let Some(composition) = master.and_then(|master| paired_composition(master, idx))
            && !codex.is_some_and(|codex| {
                validate_composition_snapshot(*fleet_id, fleet_ships, composition, codex)
            })
        {
            continue;
        }
        let reference = master.and_then(|master| master.get(idx));
        updated |= condition.apply_event_with_context(event, reference, codex);
    }
    updated
}

/// The composition the sortie condition at `idx` is fought in: the nearest
/// `Composition` before it. `KC3Kai` judges it at each battle result against the
/// fleet sent (`Kcsapi.js` battleresult, `QuestManager.isPrerequisiteFulfilled`).
pub fn paired_composition(
    conditions: &[Kc3rdQuestCondition],
    idx: usize,
) -> Option<&Kc3rdQuestConditionComposition> {
    match &conditions[paired_composition_index(conditions, idx)?] {
        Kc3rdQuestCondition::Composition(composition) => Some(composition),
        _ => None,
    }
}

/// Where [`paired_composition`] sits in `conditions`.
pub(super) fn paired_composition_index(
    conditions: &[Kc3rdQuestCondition],
    idx: usize,
) -> Option<usize> {
    if !matches!(conditions.get(idx), Some(Kc3rdQuestCondition::Sortie(_))) {
        return None;
    }
    conditions[..idx]
        .iter()
        .rposition(|condition| matches!(condition, Kc3rdQuestCondition::Composition(_)))
}

/// Whether the condition at `idx` is a composition some sortie is fought in,
/// rather than one held when the quest is claimed.
pub fn composition_is_paired(conditions: &[Kc3rdQuestCondition], idx: usize) -> bool {
    (0..conditions.len()).any(|sortie| paired_composition_index(conditions, sortie) == Some(idx))
}

fn apply_exercise_event(
    condition: &mut Kc3rdQuestConditionExercise,
    fleet_id: i64,
    win_rank: KcSortieResultRank,
    fleet_ships: &[FleetShipSnapshot],
    codex: Option<&Codex>,
) -> bool {
    if condition.times <= 0 {
        return false;
    }
    if !exercise_result_matches(&condition.expect_result, win_rank) {
        return false;
    }
    if let Some(groups) = &condition.groups {
        let Some(codex) = codex else {
            return false;
        };
        let composition = Kc3rdQuestConditionComposition {
            groups: groups.clone(),
            disallowed: None,
            fleet_id,
        };
        if !validate_composition_snapshot(fleet_id, fleet_ships, &composition, codex) {
            return false;
        }
    }

    condition.times -= 1;
    true
}

fn exercise_result_matches(required: &KcSortieResult, win_rank: KcSortieResultRank) -> bool {
    match required {
        KcSortieResult::Any => true,
        KcSortieResult::Clear => false,
        KcSortieResult::Ranked(required_rank) => win_rank <= *required_rank,
    }
}

fn apply_sortie_event(
    condition: &mut Kc3rdQuestConditionSortie,
    event: &QuestActionEvent,
    master_condition: Option<&Kc3rdQuestConditionSortie>,
) -> bool {
    // The master decides, so progress rows saved before the flag existed follow it.
    if master_condition.unwrap_or(condition).count_at_sortie_start {
        return apply_sortie_start_event(condition, event);
    }
    let (maparea_id, mapinfo_no, gauge_phase, node, fleet_id, battle) = match event {
        QuestActionEvent::SortieBattleCompleted {
            maparea_id,
            mapinfo_no,
            gauge_phase,
            boss_cell,
            node,
            win_rank,
            fleet_id,
            ..
        } => (
            *maparea_id,
            *mapinfo_no,
            *gauge_phase,
            node.as_deref(),
            *fleet_id,
            Some((*boss_cell, *win_rank)),
        ),
        QuestActionEvent::SortieNodeReached {
            maparea_id,
            mapinfo_no,
            gauge_phase,
            node,
            fleet_id,
            ..
        } => (*maparea_id, *mapinfo_no, *gauge_phase, Some(node.as_str()), *fleet_id, None),
        _ => return false,
    };
    if condition.times <= 0 {
        return false;
    }
    if condition.fleet_id > 0 && condition.fleet_id != fleet_id {
        return false;
    }
    match battle {
        Some((boss_cell, win_rank)) => {
            if condition.defeat_boss && !boss_cell {
                return false;
            }
            if !sortie_result_matches(condition.result.as_ref(), boss_cell, win_rank) {
                return false;
            }
        }
        // Reaching the cell is all an entry naming it asks, unless it asks for a win.
        None => {
            if condition.defeat_boss || matches!(condition.result, Some(KcSortieResult::Ranked(_)))
            {
                return false;
            }
        }
    }
    if let (Some(map), Some(master)) =
        (condition.map.as_mut(), master_condition.and_then(|m| m.map.as_ref()))
    {
        take_cells_from_master(map, master);
    }
    let cell = SortieCell {
        maparea_id,
        mapinfo_no,
        gauge_phase,
        node,
        reached: battle.is_none(),
    };
    if !sortie_map_matches(condition.map.as_ref(), &cell) {
        return false;
    }

    match condition.map.as_mut() {
        Some(Kc3rdQuestConditionSortieMap::All(maps)) => {
            remove_sortie_map(maps, &cell);
            if !maps.is_empty() {
                return true;
            }

            condition.times -= 1;
            if condition.times > 0 {
                reset_sortie_cycle(condition, master_condition);
            }
            true
        }
        _ => {
            condition.times -= 1;
            true
        }
    }
}

/// A sortie condition counted when the sortie starts, whatever its battles.
fn apply_sortie_start_event(
    condition: &mut Kc3rdQuestConditionSortie,
    event: &QuestActionEvent,
) -> bool {
    // ponytail: only Bw1's first requirement is counted at start, and it names
    // no map, so the map is not checked here; add it if a mapped one appears.
    let &QuestActionEvent::SortieStarted {
        fleet_id,
        ..
    } = event
    else {
        return false;
    };
    if condition.times <= 0 || (condition.fleet_id > 0 && condition.fleet_id != fleet_id) {
        return false;
    }
    condition.times -= 1;
    true
}

fn sortie_result_matches(
    required: Option<&KcSortieResult>,
    boss_cell: bool,
    win_rank: KcSortieResultRank,
) -> bool {
    match required {
        None | Some(KcSortieResult::Any) => true,
        Some(KcSortieResult::Clear) => boss_cell && win_rank <= KcSortieResultRank::B,
        Some(KcSortieResult::Ranked(required_rank)) => win_rank <= *required_rank,
    }
}

/// Where a sortie event happened.
struct SortieCell<'a> {
    maparea_id: i64,
    mapinfo_no: i64,
    gauge_phase: i64,
    node: Option<&'a str>,
    /// Reached without a battle.
    reached: bool,
}

fn sortie_map_matches(required: Option<&Kc3rdQuestConditionSortieMap>, cell: &SortieCell) -> bool {
    match required {
        // Any battle counts; a bare arrival counts only for an entry naming the cell.
        None => !cell.reached,
        Some(map) => map_infos(map).iter().any(|map| sortie_map_info_matches(map, cell)),
    }
}

/// An entry naming a gauge phase only matches battles fought in that phase, and one naming
/// a cell only what happens on that cell.
fn sortie_map_info_matches(map: &Kc3rdQuestConditionMapInfo, cell: &SortieCell) -> bool {
    map.area == cell.maparea_id
        && map.number == cell.mapinfo_no
        && map.phase.is_none_or(|phase| phase == cell.gauge_phase)
        && match map.node.as_deref() {
            Some(node) => cell.node == Some(node),
            None => !cell.reached,
        }
}

fn remove_sortie_map(maps: &mut Vec<Kc3rdQuestConditionMapInfo>, cell: &SortieCell) {
    if let Some(idx) = maps.iter().position(|map| sortie_map_info_matches(map, cell)) {
        maps.remove(idx);
    }
}

fn map_infos(map: &Kc3rdQuestConditionSortieMap) -> &[Kc3rdQuestConditionMapInfo] {
    match map {
        Kc3rdQuestConditionSortieMap::One(map) => std::slice::from_ref(map),
        Kc3rdQuestConditionSortieMap::All(maps) | Kc3rdQuestConditionSortieMap::AnyOf(maps) => maps,
    }
}

/// Progress rows saved before entries kept their cell take it from the master.
fn take_cells_from_master(
    map: &mut Kc3rdQuestConditionSortieMap,
    master: &Kc3rdQuestConditionSortieMap,
) {
    let infos = match map {
        Kc3rdQuestConditionSortieMap::One(map) => std::slice::from_mut(map),
        Kc3rdQuestConditionSortieMap::All(maps) | Kc3rdQuestConditionSortieMap::AnyOf(maps) => {
            maps.as_mut_slice()
        }
    };
    for info in infos.iter_mut().filter(|info| info.node.is_none()) {
        info.node = map_infos(master)
            .iter()
            .find(|m| m.area == info.area && m.number == info.number && m.phase == info.phase)
            .and_then(|m| m.node.clone());
    }
}

fn reset_sortie_cycle(
    condition: &mut Kc3rdQuestConditionSortie,
    master_condition: Option<&Kc3rdQuestConditionSortie>,
) {
    let Some(master_condition) = master_condition else {
        return;
    };
    condition.map = master_condition.map.clone();
}

fn apply_specific_items_event(
    items: &mut [super::Kc3rdQuestConditionSlotItem],
    item_mst_id: i64,
    stars: i64,
    codex: Option<&Codex>,
) -> bool {
    for item in items.iter_mut() {
        if item.amount <= 0 {
            continue;
        }
        if stars < item.stars {
            continue;
        }
        let matches = match &item.item_type {
            Kc3rdQuestConditionSlotItemType::Equipment(ids) => ids.contains(&item_mst_id),
            Kc3rdQuestConditionSlotItemType::EquipType(types) => {
                let Some(codex) = codex else {
                    continue;
                };
                codex
                    .manifest
                    .api_mst_slotitem
                    .iter()
                    .find(|mst| mst.api_id == item_mst_id)
                    .is_some_and(|mst| types.contains(&mst.api_type[3]))
            }
        };
        if matches {
            item.amount -= 1;
            return true;
        }
    }
    false
}

fn apply_modernization_event(
    condition: &mut super::Kc3rdQuestConditionModernization,
    codex: Option<&Codex>,
    target_ship_mst_id: i64,
    material_ship_mst_ids: &[i64],
) -> bool {
    if condition.times <= 0 {
        return false;
    }
    let target_matches = ship_matches_mst_id(&condition.target_ship, codex, target_ship_mst_id);
    if !target_matches {
        return false;
    }
    let matching_count = material_ship_mst_ids
        .iter()
        .filter(|&&mst_id| ship_matches_mst_id(&condition.material_ship, codex, mst_id))
        .count();
    if matching_count < condition.batch_size as usize {
        return false;
    }
    condition.times -= 1;
    true
}

fn ship_matches_stype(cond: &Kc3rdQuestConditionShip, codex: Option<&Codex>, stype: i64) -> bool {
    match cond {
        Kc3rdQuestConditionShip::Any => true,
        Kc3rdQuestConditionShip::ShipType(types) => types.contains(&stype),
        _ => {
            let Some(codex) = codex else {
                return false;
            };
            let Some(mst) = codex.manifest.api_mst_ship.iter().find(|m| m.api_stype == stype)
            else {
                return false;
            };
            ship_matches_mst_id(cond, Some(codex), mst.api_id)
        }
    }
}

/// Whether the ship `mst_id` is one the condition names.
pub fn ship_matches_mst_id(
    cond: &Kc3rdQuestConditionShip,
    codex: Option<&Codex>,
    mst_id: i64,
) -> bool {
    match cond {
        Kc3rdQuestConditionShip::Any => true,
        Kc3rdQuestConditionShip::Ship(ids) => ids.contains(&mst_id),
        Kc3rdQuestConditionShip::ShipType(types) => {
            let Some(codex) = codex else {
                return false;
            };
            codex
                .manifest
                .api_mst_ship
                .iter()
                .find(|m| m.api_id == mst_id)
                .is_some_and(|m| types.contains(&m.api_stype))
        }
        Kc3rdQuestConditionShip::ShipClass(classes) => {
            let Some(codex) = codex else {
                return false;
            };
            codex
                .manifest
                .api_mst_ship
                .iter()
                .find(|m| m.api_id == mst_id)
                .is_some_and(|m| classes.contains(&m.api_ctype))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(area: i64, number: i64) -> Kc3rdQuestConditionMapInfo {
        Kc3rdQuestConditionMapInfo {
            area,
            number,
            phase: None,
            node: None,
        }
    }

    fn fleet_ship(mst_id: i64, position: i64) -> FleetShipSnapshot {
        FleetShipSnapshot {
            mst_id,
            level: 1,
            position,
        }
    }

    fn first_ship_mst_by_type(codex: &Codex, ship_type: crate::kc2::KcShipType) -> i64 {
        codex
            .manifest
            .api_mst_ship
            .iter()
            .find(|mst| crate::kc2::KcShipType::n(mst.api_stype) == Some(ship_type))
            .map(|mst| mst.api_id)
            .unwrap()
    }

    #[test]
    fn sortie_event_respects_boss_and_rank_requirements() {
        let mut condition = Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
            composition: None,
            defeat_boss: true,
            fleet_id: 0,
            map: Some(Kc3rdQuestConditionSortieMap::One(map(1, 1))),
            result: Some(KcSortieResult::Ranked(KcSortieResultRank::B)),
            times: 1,
            count_at_sortie_start: false,
        });

        assert!(!condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 1,
                gauge_phase: 1,
                boss_cell: false,
                node: None,
                win_rank: KcSortieResultRank::S,
                fleet_id: 1,
                fleet_ships: vec![],
            },
            None,
        ));
        assert!(!condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 1,
                gauge_phase: 1,
                boss_cell: true,
                node: None,
                win_rank: KcSortieResultRank::C,
                fleet_id: 1,
                fleet_ships: vec![],
            },
            None,
        ));
        assert!(condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 1,
                gauge_phase: 1,
                boss_cell: true,
                node: None,
                win_rank: KcSortieResultRank::A,
                fleet_id: 1,
                fleet_ships: vec![],
            },
            None,
        ));

        let Kc3rdQuestCondition::Sortie(sortie) = condition else {
            panic!("expected sortie condition");
        };
        assert_eq!(sortie.times, 0);
    }

    #[test]
    fn sortie_clear_requires_boss_and_b_or_better() {
        let mut condition = Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
            composition: None,
            defeat_boss: false,
            fleet_id: 0,
            map: Some(Kc3rdQuestConditionSortieMap::One(map(1, 6))),
            result: Some(KcSortieResult::Clear),
            times: 1,
            count_at_sortie_start: false,
        });

        assert!(!condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 6,
                gauge_phase: 1,
                boss_cell: false,
                node: None,
                win_rank: KcSortieResultRank::A,
                fleet_id: 1,
                fleet_ships: vec![],
            },
            None,
        ));
        assert!(!condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 6,
                gauge_phase: 1,
                boss_cell: true,
                node: None,
                win_rank: KcSortieResultRank::C,
                fleet_id: 1,
                fleet_ships: vec![],
            },
            None,
        ));
        assert!(condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 6,
                gauge_phase: 1,
                boss_cell: true,
                node: None,
                win_rank: KcSortieResultRank::B,
                fleet_id: 1,
                fleet_ships: vec![],
            },
            None,
        ));
    }

    #[test]
    fn sortie_all_map_cycles_reset_from_master_requirement() {
        let master = Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
            composition: None,
            defeat_boss: true,
            fleet_id: 0,
            map: Some(Kc3rdQuestConditionSortieMap::All(vec![map(1, 1), map(1, 2)])),
            result: Some(KcSortieResult::Ranked(KcSortieResultRank::B)),
            times: 2,
            count_at_sortie_start: false,
        });
        let mut current = master.clone();

        assert!(current.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 1,
                gauge_phase: 1,
                boss_cell: true,
                node: None,
                win_rank: KcSortieResultRank::A,
                fleet_id: 1,
                fleet_ships: vec![],
            },
            Some(&master),
        ));
        let Kc3rdQuestCondition::Sortie(sortie) = &current else {
            panic!("expected sortie condition");
        };
        assert_eq!(sortie.times, 2);
        assert_eq!(sortie.map, Some(Kc3rdQuestConditionSortieMap::All(vec![map(1, 2)])),);

        assert!(current.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 2,
                gauge_phase: 1,
                boss_cell: true,
                node: None,
                win_rank: KcSortieResultRank::A,
                fleet_id: 1,
                fleet_ships: vec![],
            },
            Some(&master),
        ));
        let Kc3rdQuestCondition::Sortie(sortie) = &current else {
            panic!("expected sortie condition");
        };
        assert_eq!(sortie.times, 1);
        assert_eq!(sortie.map, Some(Kc3rdQuestConditionSortieMap::All(vec![map(1, 1), map(1, 2)])));
    }

    fn phased(area: i64, number: i64, phase: i64) -> Kc3rdQuestConditionMapInfo {
        Kc3rdQuestConditionMapInfo {
            area,
            number,
            phase: Some(phase),
            node: None,
        }
    }

    /// An S-rank boss win on `area-number` fought in `gauge_phase`.
    fn boss_s(area: i64, number: i64, gauge_phase: i64) -> QuestActionEvent {
        QuestActionEvent::SortieBattleCompleted {
            maparea_id: area,
            mapinfo_no: number,
            gauge_phase,
            boss_cell: true,
            node: None,
            win_rank: KcSortieResultRank::S,
            fleet_id: 1,
            fleet_ships: vec![],
        }
    }

    fn at_cell(area: i64, number: i64, node: &str) -> Kc3rdQuestConditionMapInfo {
        Kc3rdQuestConditionMapInfo {
            node: Some(node.to_string()),
            ..map(area, number)
        }
    }

    /// The fleet reached `area-number` cell `node` without fighting there.
    fn reached(area: i64, number: i64, node: &str) -> QuestActionEvent {
        QuestActionEvent::SortieNodeReached {
            maparea_id: area,
            mapinfo_no: number,
            gauge_phase: 1,
            node: node.to_string(),
            fleet_id: 1,
            fleet_ships: vec![],
        }
    }

    /// A sortie entry on `map`, once, with `result`.
    fn entry(
        map: Option<Kc3rdQuestConditionMapInfo>,
        result: Option<KcSortieResult>,
    ) -> Kc3rdQuestCondition {
        Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
            map: map.map(Kc3rdQuestConditionSortieMap::One),
            result,
            times: 1,
            ..Default::default()
        })
    }

    fn times_left(condition: &Kc3rdQuestCondition) -> i64 {
        match condition {
            Kc3rdQuestCondition::Sortie(sortie) => sortie.times,
            other => panic!("expected a sortie condition, got {other:?}"),
        }
    }

    /// 807's 1-6 entry: reaching the goal is the whole of it (#149).
    #[test]
    fn reaching_1_6_n_counts_an_entry_naming_it() {
        let master = entry(Some(at_cell(1, 6, "N")), Some(KcSortieResult::Clear));

        let mut elsewhere = master.clone();
        assert!(!elsewhere.apply_event_with_reference(&reached(1, 6, "M"), Some(&master)));
        let mut fought = master.clone();
        assert!(!fought.apply_event_with_reference(&boss_s(1, 6, 1), Some(&master)));

        let mut current = master.clone();
        assert!(current.apply_event_with_reference(&reached(1, 6, "N"), Some(&master)));
        assert_eq!(times_left(&current), 0);
    }

    #[test]
    fn reaching_a_cell_counts_no_entry_that_names_none() {
        for master in [entry(Some(map(1, 6)), None), entry(None, None)] {
            let mut current = master.clone();
            assert!(!current.apply_event_with_reference(&reached(1, 6, "N"), Some(&master)));
        }
    }

    #[test]
    fn reaching_a_cell_counts_no_entry_asking_for_a_win() {
        let master =
            entry(Some(at_cell(1, 6, "N")), Some(KcSortieResult::Ranked(KcSortieResultRank::A)));
        let mut current = master.clone();
        assert!(!current.apply_event_with_reference(&reached(1, 6, "N"), Some(&master)));
    }

    /// 847's 7-3P: a boss win counts only on cell P.
    #[test]
    fn a_battle_counts_an_entry_naming_a_cell_only_on_that_cell() {
        let master =
            entry(Some(at_cell(7, 3, "P")), Some(KcSortieResult::Ranked(KcSortieResultRank::S)));
        let on = |node: &str| match boss_s(7, 3, 2) {
            QuestActionEvent::SortieBattleCompleted {
                maparea_id,
                mapinfo_no,
                gauge_phase,
                boss_cell,
                win_rank,
                fleet_id,
                fleet_ships,
                ..
            } => QuestActionEvent::SortieBattleCompleted {
                maparea_id,
                mapinfo_no,
                gauge_phase,
                boss_cell,
                node: Some(node.to_string()),
                win_rank,
                fleet_id,
                fleet_ships,
            },
            _ => unreachable!(),
        };

        let mut current = master.clone();
        assert!(!current.apply_event_with_reference(&on("E"), Some(&master)));
        assert!(current.apply_event_with_reference(&on("P"), Some(&master)));
    }

    /// 855's shape: boss S wins on 1-3, 1-4 and 1-5, and 1-6's goal. Reaching the goal
    /// leaves the boss wins to do.
    #[test]
    fn reaching_the_goal_leaves_the_other_entries_of_the_quest() {
        let master = vec![
            Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
                defeat_boss: true,
                map: Some(Kc3rdQuestConditionSortieMap::All(vec![map(1, 3), map(1, 4), map(1, 5)])),
                result: Some(KcSortieResult::Ranked(KcSortieResultRank::S)),
                times: 1,
                ..Default::default()
            }),
            entry(Some(at_cell(1, 6, "N")), Some(KcSortieResult::Clear)),
        ];
        let mut current = master.clone();

        assert!(apply_event_to_conditions(&mut current, Some(&master), &reached(1, 6, "N"), None));

        assert_eq!(current.iter().map(times_left).collect::<Vec<_>>(), [1, 0]);
    }

    /// A progress row saved before entries kept their cell takes it from the master.
    #[test]
    fn an_old_progress_row_takes_the_cell_from_the_master() {
        let master = entry(Some(at_cell(1, 6, "N")), Some(KcSortieResult::Clear));
        let mut old_row = entry(Some(map(1, 6)), Some(KcSortieResult::Clear));

        assert!(old_row.apply_event_with_reference(&reached(1, 6, "N"), Some(&master)));
        assert_eq!(times_left(&old_row), 0);
    }

    /// 893's shape: 1-5, 7-1, 7-2 phase 1 and 7-2 phase 2, three cycles.
    fn quest_893() -> Kc3rdQuestCondition {
        Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
            composition: None,
            defeat_boss: true,
            fleet_id: 0,
            map: Some(Kc3rdQuestConditionSortieMap::All(vec![
                map(1, 5),
                map(7, 1),
                phased(7, 2, 1),
                phased(7, 2, 2),
            ])),
            result: Some(KcSortieResult::Ranked(KcSortieResultRank::S)),
            times: 3,
            count_at_sortie_start: false,
        })
    }

    fn remaining_maps(condition: &Kc3rdQuestCondition) -> Vec<Kc3rdQuestConditionMapInfo> {
        match condition {
            Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
                map: Some(Kc3rdQuestConditionSortieMap::All(maps)),
                ..
            }) => maps.clone(),
            other => panic!("expected an All sortie condition, got {other:?}"),
        }
    }

    #[test]
    fn a_phase_2_win_on_7_2_removes_only_the_phase_2_entry() {
        let master = quest_893();
        let mut current = master.clone();

        assert!(current.apply_event_with_reference(&boss_s(7, 2, 2), Some(&master)));

        assert_eq!(
            remaining_maps(&current),
            vec![map(1, 5), map(7, 1), phased(7, 2, 1)],
            "the phase 1 entry stays"
        );
    }

    #[test]
    fn a_phase_1_win_on_7_2_removes_only_the_phase_1_entry() {
        let master = quest_893();
        let mut current = master.clone();

        assert!(current.apply_event_with_reference(&boss_s(7, 2, 1), Some(&master)));
        assert_eq!(remaining_maps(&current), vec![map(1, 5), map(7, 1), phased(7, 2, 2)]);

        // A second phase 1 win finds nothing left to remove.
        assert!(!current.apply_event_with_reference(&boss_s(7, 2, 1), Some(&master)));
    }

    #[test]
    fn a_map_entry_without_a_phase_matches_any_phase() {
        let master = quest_893();
        let mut current = master.clone();

        assert!(current.apply_event_with_reference(&boss_s(7, 1, 3), Some(&master)));
        assert_eq!(remaining_maps(&current), vec![map(1, 5), phased(7, 2, 1), phased(7, 2, 2)]);
    }

    #[test]
    fn quest_893_completes_after_three_cycles_of_its_four_entries() {
        let master = quest_893();
        let mut current = master.clone();

        for _ in 0..3 {
            for event in [boss_s(1, 5, 1), boss_s(7, 1, 1), boss_s(7, 2, 1), boss_s(7, 2, 2)] {
                assert!(current.apply_event_with_reference(&event, Some(&master)));
            }
        }

        let Kc3rdQuestCondition::Sortie(sortie) = current else {
            panic!("expected sortie condition");
        };
        assert_eq!(sortie.times, 0);
    }

    #[test]
    fn a_sortie_start_counts_only_conditions_flagged_for_it() {
        let flagged = Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
            times: 2,
            count_at_sortie_start: true,
            ..Default::default()
        });
        let plain = Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
            times: 2,
            ..Default::default()
        });
        let start = QuestActionEvent::SortieStarted {
            maparea_id: 1,
            mapinfo_no: 1,
            fleet_id: 1,
        };
        let battle = boss_s(1, 1, 1);

        let mut current = flagged.clone();
        assert!(current.apply_event_with_reference(&start, Some(&flagged)));
        assert!(!current.apply_event_with_reference(&battle, Some(&flagged)));

        // A row saved without the flag still follows its master.
        let mut old_row = plain.clone();
        assert!(old_row.apply_event_with_reference(&start, Some(&flagged)));

        let mut current = plain.clone();
        assert!(!current.apply_event_with_reference(&start, Some(&plain)));
        assert!(current.apply_event_with_reference(&battle, Some(&plain)));
    }

    #[test]
    fn exercise_event_respects_result_requirement() {
        let mut condition = Kc3rdQuestCondition::Exercise(Kc3rdQuestConditionExercise {
            times: 2,
            expect_result: KcSortieResult::Ranked(KcSortieResultRank::B),
            expire_next_day: false,
            groups: None,
        });

        assert!(!condition.apply_event_with_context(
            &QuestActionEvent::ExerciseBattleCompleted {
                fleet_id: 1,
                win_rank: KcSortieResultRank::C,
                fleet_ships: vec![fleet_ship(1, 1)],
            },
            None,
            None,
        ));
        assert!(condition.apply_event_with_context(
            &QuestActionEvent::ExerciseBattleCompleted {
                fleet_id: 1,
                win_rank: KcSortieResultRank::B,
                fleet_ships: vec![fleet_ship(1, 1)],
            },
            None,
            None,
        ));

        let Kc3rdQuestCondition::Exercise(exercise) = condition else {
            panic!("expected exercise condition");
        };
        assert_eq!(exercise.times, 1);
    }

    #[test]
    fn exercise_event_validates_group_requirements_with_snapshot() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, crate::kc2::KcShipType::DD);
        let mut condition = Kc3rdQuestCondition::Exercise(Kc3rdQuestConditionExercise {
            times: 1,
            expect_result: KcSortieResult::Ranked(KcSortieResultRank::B),
            expire_next_day: true,
            groups: Some(vec![
                crate::thirdparty::Kc3rdQuestConditionShipGroup {
                    ship: crate::thirdparty::Kc3rdQuestConditionShip::ShipType(vec![2]),
                    amount: crate::thirdparty::Kc3rdQuestShipAmount::exact(4),
                    lv: 0,
                    position: 0,
                    other_ships: false,
                    white_list: None,
                },
                crate::thirdparty::Kc3rdQuestConditionShipGroup {
                    ship: crate::thirdparty::Kc3rdQuestConditionShip::Any,
                    amount: crate::thirdparty::Kc3rdQuestShipAmount::range(0, 2),
                    lv: 0,
                    position: 0,
                    other_ships: false,
                    white_list: None,
                },
            ]),
        });

        assert!(condition.apply_event_with_context(
            &QuestActionEvent::ExerciseBattleCompleted {
                fleet_id: 1,
                win_rank: KcSortieResultRank::A,
                fleet_ships: vec![
                    fleet_ship(dd_mst, 1),
                    fleet_ship(dd_mst, 2),
                    fleet_ship(dd_mst, 3),
                    fleet_ship(dd_mst, 4),
                ],
            },
            None,
            Some(&codex),
        ));
    }

    fn composition(fleet_id: i64) -> Kc3rdQuestCondition {
        Kc3rdQuestCondition::Composition(Kc3rdQuestConditionComposition {
            groups: vec![],
            disallowed: None,
            fleet_id,
        })
    }

    fn sortie(times: i64) -> Kc3rdQuestCondition {
        Kc3rdQuestCondition::Sortie(Kc3rdQuestConditionSortie {
            times,
            ..Default::default()
        })
    }

    fn exercise() -> Kc3rdQuestCondition {
        Kc3rdQuestCondition::Exercise(Kc3rdQuestConditionExercise {
            times: 1,
            expect_result: KcSortieResult::Any,
            expire_next_day: false,
            groups: None,
        })
    }

    fn fleet_of(condition: Option<&Kc3rdQuestConditionComposition>) -> Option<i64> {
        condition.map(|c| c.fleet_id)
    }

    #[test]
    fn one_composition_binds_every_sortie_after_it() {
        let conditions = [composition(1), sortie(1), sortie(2)];
        assert_eq!(fleet_of(paired_composition(&conditions, 1)), Some(1));
        assert_eq!(fleet_of(paired_composition(&conditions, 2)), Some(1));
        assert_eq!(fleet_of(paired_composition(&conditions, 0)), None, "not a sortie");
    }

    #[test]
    fn each_composition_binds_the_sortie_that_follows_it() {
        let conditions = [composition(1), sortie(1), composition(2), sortie(1)];
        assert_eq!(fleet_of(paired_composition(&conditions, 1)), Some(1));
        assert_eq!(fleet_of(paired_composition(&conditions, 3)), Some(2));
    }

    #[test]
    fn an_exercise_before_the_composition_is_not_bound() {
        let conditions = [exercise(), composition(1), sortie(1)];
        assert_eq!(fleet_of(paired_composition(&conditions, 2)), Some(1));
        assert!(!composition_is_paired(&conditions, 0));
        assert!(composition_is_paired(&conditions, 1));
    }

    #[test]
    fn a_sortie_without_a_composition_is_not_bound() {
        let conditions = [sortie(1), composition(1)];
        assert_eq!(fleet_of(paired_composition(&conditions, 0)), None);
        assert!(!composition_is_paired(&conditions, 1), "a composition held, not sailed");
    }

    #[test]
    fn a_bound_sortie_counts_only_battles_fought_in_its_composition() {
        let master = [composition(2), sortie(2), sortie(2)];
        let mut current = master.clone();
        let battle = |fleet_id| QuestActionEvent::SortieBattleCompleted {
            maparea_id: 1,
            mapinfo_no: 1,
            gauge_phase: 1,
            boss_cell: true,
            node: None,
            win_rank: KcSortieResultRank::S,
            fleet_id,
            fleet_ships: vec![],
        };

        let codex = Some(&Codex::default());

        assert!(!apply_event_to_conditions(&mut current, Some(&master), &battle(1), codex));
        assert_eq!(current, master, "fleet 1 is not the composition's fleet 2");
        assert!(apply_event_to_conditions(&mut current, Some(&master), &battle(2), codex));
        assert_eq!(current[1], sortie(1));
        assert_eq!(current[2], sortie(1));
    }

    // --- SpecificItems tests ---

    #[test]
    fn specific_items_matches_slot_item_scrapped() {
        let cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(vec![]));
        let event = QuestActionEvent::SlotItemScrapped {
            item_mst_id: 42,
            stars: 0,
        };
        assert!(cond.matches_event(&event));
    }

    #[test]
    fn specific_items_apply_equipment_match() {
        use crate::thirdparty::{Kc3rdQuestConditionSlotItem, Kc3rdQuestConditionSlotItemType};
        let mut cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(vec![
            Kc3rdQuestConditionSlotItem {
                item_type: Kc3rdQuestConditionSlotItemType::Equipment(vec![42, 43]),
                amount: 2,
                stars: 0,
                fully_skilled: false,
            },
        ]));
        assert!(cond.apply_event(&QuestActionEvent::SlotItemScrapped {
            item_mst_id: 42,
            stars: 0,
        }));
        let Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(items)) = cond
        else {
            panic!("expected specific items");
        };
        assert_eq!(items[0].amount, 1);
    }

    #[test]
    fn specific_items_apply_equipment_no_match() {
        use crate::thirdparty::{Kc3rdQuestConditionSlotItem, Kc3rdQuestConditionSlotItemType};
        let mut cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(vec![
            Kc3rdQuestConditionSlotItem {
                item_type: Kc3rdQuestConditionSlotItemType::Equipment(vec![42]),
                amount: 1,
                stars: 0,
                fully_skilled: false,
            },
        ]));
        assert!(!cond.apply_event(&QuestActionEvent::SlotItemScrapped {
            item_mst_id: 99,
            stars: 0,
        }));
    }

    #[test]
    fn specific_items_apply_stars_threshold() {
        use crate::thirdparty::{Kc3rdQuestConditionSlotItem, Kc3rdQuestConditionSlotItemType};
        let mut cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(vec![
            Kc3rdQuestConditionSlotItem {
                item_type: Kc3rdQuestConditionSlotItemType::Equipment(vec![42]),
                amount: 1,
                stars: 5,
                fully_skilled: false,
            },
        ]));
        // stars too low
        assert!(!cond.apply_event(&QuestActionEvent::SlotItemScrapped {
            item_mst_id: 42,
            stars: 3,
        }));
        // stars meets threshold
        assert!(cond.apply_event(&QuestActionEvent::SlotItemScrapped {
            item_mst_id: 42,
            stars: 5,
        }));
    }

    #[test]
    fn specific_items_apply_equip_type_with_codex() {
        use crate::thirdparty::{Kc3rdQuestConditionSlotItem, Kc3rdQuestConditionSlotItemType};
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        // Find a slot item and its api_type[3]
        let some_item = codex.manifest.api_mst_slotitem.first().unwrap();
        let type3 = some_item.api_type[3];
        let item_id = some_item.api_id;

        let mut cond = Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(vec![
            Kc3rdQuestConditionSlotItem {
                item_type: Kc3rdQuestConditionSlotItemType::EquipType(vec![type3]),
                amount: 1,
                stars: 0,
                fully_skilled: false,
            },
        ]));
        assert!(cond.apply_event_with_context(
            &QuestActionEvent::SlotItemScrapped {
                item_mst_id: item_id,
                stars: 0,
            },
            None,
            Some(&codex),
        ));
    }

    // --- Modernization tests ---

    #[test]
    fn modernization_matches_event() {
        let cond =
            Kc3rdQuestCondition::Modernization(super::super::Kc3rdQuestConditionModernization {
                target_ship: Kc3rdQuestConditionShip::Any,
                material_ship: Kc3rdQuestConditionShip::Any,
                batch_size: 2,
                times: 1,
            });
        let event = QuestActionEvent::ModernizationCompleted {
            target_ship_mst_id: 1,
            material_ship_mst_ids: vec![2, 3],
        };
        assert!(cond.matches_event(&event));
    }

    #[test]
    fn modernization_apply_requires_enough_matching_materials() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, crate::kc2::KcShipType::DD);
        let cl_mst = first_ship_mst_by_type(&codex, crate::kc2::KcShipType::CL);

        let mut cond =
            Kc3rdQuestCondition::Modernization(super::super::Kc3rdQuestConditionModernization {
                target_ship: Kc3rdQuestConditionShip::Any,
                material_ship: Kc3rdQuestConditionShip::ShipType(vec![2]), // DD
                batch_size: 2,
                times: 1,
            });

        // Only 1 DD out of 4 materials — should NOT progress
        assert!(!cond.apply_event_with_context(
            &QuestActionEvent::ModernizationCompleted {
                target_ship_mst_id: cl_mst,
                material_ship_mst_ids: vec![dd_mst, cl_mst, cl_mst, cl_mst],
            },
            None,
            Some(&codex),
        ));

        // 2 DDs out of 4 materials — should progress
        assert!(cond.apply_event_with_context(
            &QuestActionEvent::ModernizationCompleted {
                target_ship_mst_id: cl_mst,
                material_ship_mst_ids: vec![dd_mst, dd_mst, cl_mst, cl_mst],
            },
            None,
            Some(&codex),
        ));
    }

    #[test]
    fn modernization_apply_validates_target_ship() {
        let codex = Codex::load_without_cache_source("../../.data/codex").unwrap();
        let dd_mst = first_ship_mst_by_type(&codex, crate::kc2::KcShipType::DD);
        let cl_mst = first_ship_mst_by_type(&codex, crate::kc2::KcShipType::CL);

        let mut cond =
            Kc3rdQuestCondition::Modernization(super::super::Kc3rdQuestConditionModernization {
                target_ship: Kc3rdQuestConditionShip::ShipType(vec![2]), // DD only
                material_ship: Kc3rdQuestConditionShip::Any,
                batch_size: 1,
                times: 1,
            });

        // Target is CL, quest requires DD target — should fail
        assert!(!cond.apply_event_with_context(
            &QuestActionEvent::ModernizationCompleted {
                target_ship_mst_id: cl_mst,
                material_ship_mst_ids: vec![dd_mst],
            },
            None,
            Some(&codex),
        ));

        // Target is DD — should succeed
        assert!(cond.apply_event_with_context(
            &QuestActionEvent::ModernizationCompleted {
                target_ship_mst_id: dd_mst,
                material_ship_mst_ids: vec![dd_mst],
            },
            None,
            Some(&codex),
        ));
    }

    // --- Sink tests ---

    #[test]
    fn sink_matches_enemy_ship_sunk() {
        let cond = Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::ShipType(vec![11]), 3);
        let event = QuestActionEvent::EnemyShipSunk {
            ship_stype: 11,
        };
        assert!(cond.matches_event(&event));
    }

    #[test]
    fn sink_apply_decrements_on_stype_match() {
        let mut cond = Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::ShipType(vec![11]), 3);
        assert!(cond.apply_event(&QuestActionEvent::EnemyShipSunk {
            ship_stype: 11
        }));
        let Kc3rdQuestCondition::Sink(_, count) = cond else {
            panic!("expected sink")
        };
        assert_eq!(count, 2);
    }

    #[test]
    fn sink_apply_no_decrement_on_stype_mismatch() {
        let mut cond = Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::ShipType(vec![11]), 3);
        assert!(!cond.apply_event(&QuestActionEvent::EnemyShipSunk {
            ship_stype: 2
        }));
    }

    #[test]
    fn sink_apply_any_matches_all() {
        let mut cond = Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::Any, 2);
        assert!(cond.apply_event(&QuestActionEvent::EnemyShipSunk {
            ship_stype: 99
        }));
        let Kc3rdQuestCondition::Sink(_, count) = cond else {
            panic!("expected sink")
        };
        assert_eq!(count, 1);
    }

    // --- SlotItemImproved tests ---

    #[test]
    fn slot_item_improvement_matches_slot_item_improved() {
        let cond = Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(3));
        let event = QuestActionEvent::SlotItemImproved {
            item_mst_id: 42,
            stars: 5,
        };
        assert!(cond.matches_event(&event));
    }

    #[test]
    fn slot_item_improvement_apply_decrements() {
        let mut cond =
            Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(2));
        assert!(cond.apply_event(&QuestActionEvent::SlotItemImproved {
            item_mst_id: 42,
            stars: 5,
        }));
        assert_eq!(
            cond,
            Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(1))
        );
    }

    #[test]
    fn slot_item_improvement_no_decrement_at_zero() {
        let mut cond =
            Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(0));
        assert!(!cond.apply_event(&QuestActionEvent::SlotItemImproved {
            item_mst_id: 42,
            stars: 5,
        }));
    }
}
