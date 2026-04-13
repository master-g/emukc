//! Quest condition matching

use crate::codex::Codex;
use crate::kc2::{KcSortieResult, KcSortieResultRank};
use crate::thirdparty::{
    FleetShipSnapshot, Kc3rdQuestCondition, Kc3rdQuestConditionComposition,
    Kc3rdQuestConditionExercise, Kc3rdQuestConditionFactory, Kc3rdQuestConditionMapInfo,
    Kc3rdQuestConditionScrap, Kc3rdQuestConditionSortie, Kc3rdQuestConditionSortieMap,
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
            ) | (Kc3rdQuestCondition::Repair(_), QuestActionEvent::ShipRepaired { .. },)
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
}
