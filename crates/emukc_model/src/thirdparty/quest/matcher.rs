//! Quest condition matching

use crate::codex::Codex;
use crate::kc2::{KcSortieResult, KcSortieResultRank};
use crate::thirdparty::{
    FleetShipSnapshot, Kc3rdQuestCondition, Kc3rdQuestConditionComposition,
    Kc3rdQuestConditionExercise, Kc3rdQuestConditionFactory, Kc3rdQuestConditionMapInfo,
    Kc3rdQuestConditionScrap, Kc3rdQuestConditionShip, Kc3rdQuestConditionSlotItemType,
    Kc3rdQuestConditionSortie, Kc3rdQuestConditionSortieMap,
    validate_composition_snapshot,
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
    SortieBattleCompleted {
        maparea_id: i64,
        mapinfo_no: i64,
        boss_cell: bool,
        win_rank: KcSortieResultRank,
        fleet_id: i64,
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
            )
            | (
                Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(_)),
                QuestActionEvent::SlotItemScrapped { .. },
            )
            | (Kc3rdQuestCondition::Modernization(_), QuestActionEvent::ModernizationCompleted { .. })
            | (Kc3rdQuestCondition::Sink(_, _), QuestActionEvent::EnemyShipSunk { .. })
            | (
                Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(_)),
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
                | Kc3rdQuestConditionFactory::SlotItemConstruction(count),
            )
            | Kc3rdQuestCondition::Scrap(
                Kc3rdQuestConditionScrap::AnyShip(count)
                | Kc3rdQuestConditionScrap::AnyEquipment(count),
            )
            | Kc3rdQuestCondition::Repair(count)
            | Kc3rdQuestCondition::Resupply(count) => {
                if *count > 0 {
                    *count -= 1;
                    true
                } else {
                    false
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

                for condition in conditions.iter_mut() {
                    let matches = condition.list.as_ref().is_none_or(|allowed_ids| {
                        let mission_id_str = mission_id.to_string();
                        allowed_ids.iter().any(|id| {
                            id == &mission_id_str
                                || id.parse::<i64>().ok().is_some_and(|v| v == *mission_id)
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
                let QuestActionEvent::SortieBattleCompleted {
                    maparea_id,
                    mapinfo_no,
                    boss_cell,
                    win_rank,
                    fleet_id,
                } = event
                else {
                    return false;
                };

                let Some(master_condition) = reference.and_then(|condition| match condition {
                    Kc3rdQuestCondition::Sortie(sortie) => Some(sortie),
                    _ => None,
                }) else {
                    return apply_sortie_event(
                        condition,
                        *maparea_id,
                        *mapinfo_no,
                        *boss_cell,
                        *win_rank,
                        *fleet_id,
                        None,
                    );
                };

                apply_sortie_event(
                    condition,
                    *maparea_id,
                    *mapinfo_no,
                    *boss_cell,
                    *win_rank,
                    *fleet_id,
                    Some(master_condition),
                )
            }
            Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(items)) => {
                let QuestActionEvent::SlotItemScrapped { item_mst_id, stars } = event else {
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
                apply_modernization_event(condition, codex, *target_ship_mst_id, material_ship_mst_ids)
            }
            Kc3rdQuestCondition::Sink(ship_cond, count) => {
                let QuestActionEvent::EnemyShipSunk { ship_stype } = event else {
                    return false;
                };
                if *count > 0 && ship_matches_stype(ship_cond, codex, *ship_stype) {
                    *count -= 1;
                    true
                } else {
                    false
                }
            }
            Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(count)) => {
                if *count > 0 {
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
    maparea_id: i64,
    mapinfo_no: i64,
    boss_cell: bool,
    win_rank: KcSortieResultRank,
    fleet_id: i64,
    master_condition: Option<&Kc3rdQuestConditionSortie>,
) -> bool {
    if condition.times <= 0 {
        return false;
    }
    if condition.fleet_id > 0 && condition.fleet_id != fleet_id {
        return false;
    }
    if condition.defeat_boss && !boss_cell {
        return false;
    }
    if !sortie_result_matches(condition.result.as_ref(), boss_cell, win_rank) {
        return false;
    }
    if !sortie_map_matches(condition.map.as_ref(), maparea_id, mapinfo_no) {
        return false;
    }

    match condition.map.as_mut() {
        Some(Kc3rdQuestConditionSortieMap::All(maps)) => {
            remove_sortie_map(maps, maparea_id, mapinfo_no);
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

fn sortie_map_matches(
    required: Option<&Kc3rdQuestConditionSortieMap>,
    maparea_id: i64,
    mapinfo_no: i64,
) -> bool {
    match required {
        None => true,
        Some(Kc3rdQuestConditionSortieMap::One(map)) => {
            sortie_map_info_matches(map, maparea_id, mapinfo_no)
        }
        Some(Kc3rdQuestConditionSortieMap::AnyOf(maps))
        | Some(Kc3rdQuestConditionSortieMap::All(maps)) => {
            maps.iter().any(|map| sortie_map_info_matches(map, maparea_id, mapinfo_no))
        }
    }
}

fn sortie_map_info_matches(
    map: &Kc3rdQuestConditionMapInfo,
    maparea_id: i64,
    mapinfo_no: i64,
) -> bool {
    map.phase.is_none() && map.area == maparea_id && map.number == mapinfo_no
}

fn remove_sortie_map(maps: &mut Vec<Kc3rdQuestConditionMapInfo>, maparea_id: i64, mapinfo_no: i64) {
    if let Some(idx) =
        maps.iter().position(|map| sortie_map_info_matches(map, maparea_id, mapinfo_no))
    {
        maps.remove(idx);
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
            let Some(mst) = codex
                .manifest
                .api_mst_ship
                .iter()
                .find(|m| m.api_stype == stype)
            else {
                return false;
            };
            ship_matches_mst_id(cond, Some(codex), mst.api_id)
        }
    }
}

fn ship_matches_mst_id(cond: &Kc3rdQuestConditionShip, codex: Option<&Codex>, mst_id: i64) -> bool {
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
                .is_some_and(|m| types.contains(&(m.api_stype as i64)))
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
                .is_some_and(|m| classes.contains(&(m.api_ctype as i64)))
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
        });

        assert!(!condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 1,
                boss_cell: false,
                win_rank: KcSortieResultRank::S,
                fleet_id: 1,
            },
            None,
        ));
        assert!(!condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 1,
                boss_cell: true,
                win_rank: KcSortieResultRank::C,
                fleet_id: 1,
            },
            None,
        ));
        assert!(condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 1,
                boss_cell: true,
                win_rank: KcSortieResultRank::A,
                fleet_id: 1,
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
        });

        assert!(!condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 6,
                boss_cell: false,
                win_rank: KcSortieResultRank::A,
                fleet_id: 1,
            },
            None,
        ));
        assert!(!condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 6,
                boss_cell: true,
                win_rank: KcSortieResultRank::C,
                fleet_id: 1,
            },
            None,
        ));
        assert!(condition.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 6,
                boss_cell: true,
                win_rank: KcSortieResultRank::B,
                fleet_id: 1,
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
        });
        let mut current = master.clone();

        assert!(current.apply_event_with_reference(
            &QuestActionEvent::SortieBattleCompleted {
                maparea_id: 1,
                mapinfo_no: 1,
                boss_cell: true,
                win_rank: KcSortieResultRank::A,
                fleet_id: 1,
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
                boss_cell: true,
                win_rank: KcSortieResultRank::A,
                fleet_id: 1,
            },
            Some(&master),
        ));
        let Kc3rdQuestCondition::Sortie(sortie) = &current else {
            panic!("expected sortie condition");
        };
        assert_eq!(sortie.times, 1);
        assert_eq!(sortie.map, Some(Kc3rdQuestConditionSortieMap::All(vec![map(1, 1), map(1, 2)])));
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
        let cond = Kc3rdQuestCondition::Modernization(super::super::Kc3rdQuestConditionModernization {
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

        let mut cond = Kc3rdQuestCondition::Modernization(super::super::Kc3rdQuestConditionModernization {
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

        let mut cond = Kc3rdQuestCondition::Modernization(super::super::Kc3rdQuestConditionModernization {
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
        let event = QuestActionEvent::EnemyShipSunk { ship_stype: 11 };
        assert!(cond.matches_event(&event));
    }

    #[test]
    fn sink_apply_decrements_on_stype_match() {
        let mut cond = Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::ShipType(vec![11]), 3);
        assert!(cond.apply_event(&QuestActionEvent::EnemyShipSunk { ship_stype: 11 }));
        let Kc3rdQuestCondition::Sink(_, count) = cond else { panic!("expected sink") };
        assert_eq!(count, 2);
    }

    #[test]
    fn sink_apply_no_decrement_on_stype_mismatch() {
        let mut cond = Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::ShipType(vec![11]), 3);
        assert!(!cond.apply_event(&QuestActionEvent::EnemyShipSunk { ship_stype: 2 }));
    }

    #[test]
    fn sink_apply_any_matches_all() {
        let mut cond = Kc3rdQuestCondition::Sink(Kc3rdQuestConditionShip::Any, 2);
        assert!(cond.apply_event(&QuestActionEvent::EnemyShipSunk { ship_stype: 99 }));
        let Kc3rdQuestCondition::Sink(_, count) = cond else { panic!("expected sink") };
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
        let mut cond = Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(2));
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
        let mut cond = Kc3rdQuestCondition::Factory(Kc3rdQuestConditionFactory::SlotItemImprovement(0));
        assert!(!cond.apply_event(&QuestActionEvent::SlotItemImproved {
            item_mst_id: 42,
            stars: 5,
        }));
    }
}
