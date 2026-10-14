#![allow(missing_docs)]

use crate::profile::quest::QuestProgressStatus;

use super::{
    Kc3rdQuestCondition, Kc3rdQuestConditionFactory, Kc3rdQuestConditionScrap,
    Kc3rdQuestRequirement,
    matcher::{composition_is_paired, paired_composition_index},
};

impl Kc3rdQuestRequirement {
    pub fn calculate_progress(&self, mst: &Kc3rdQuestRequirement) -> QuestProgressStatus {
        let (Kc3rdQuestRequirement::And(master)
        | Kc3rdQuestRequirement::OneOf(master)
        | Kc3rdQuestRequirement::Sequential(master)) = mst;
        let satisfied_at =
            |conditions: &[Kc3rdQuestCondition], idx| satisfied_at(conditions, master, idx);
        match self {
            Kc3rdQuestRequirement::And(conditions) => {
                // Each expedition an `Expedition` condition lists counts on its own.
                let (total, completed) = conditions
                    .iter()
                    .enumerate()
                    .map(|(idx, c)| match c {
                        Kc3rdQuestCondition::Expedition(exps) => {
                            (exps.len(), exps.iter().filter(|e| e.times == 0).count())
                        }
                        _ => (1, usize::from(satisfied_at(conditions, idx))),
                    })
                    .fold((0, 0), |(total, completed), (units, done)| {
                        (total + units, completed + done)
                    });
                progress_from_ratio(completed, total)
            }
            Kc3rdQuestRequirement::OneOf(conditions) => {
                if (0..conditions.len()).any(|idx| satisfied_at(conditions, idx)) {
                    QuestProgressStatus::Completed
                } else {
                    QuestProgressStatus::Empty
                }
            }
            Kc3rdQuestRequirement::Sequential(conditions) => {
                let first_incomplete =
                    (0..conditions.len()).position(|idx| !satisfied_at(conditions, idx));
                match first_incomplete {
                    None => QuestProgressStatus::Completed,
                    Some(idx) => progress_from_ratio(idx, conditions.len()),
                }
            }
        }
    }
}

impl Kc3rdQuestCondition {
    pub fn is_satisfied(&self) -> bool {
        match self {
            Kc3rdQuestCondition::Factory(
                Kc3rdQuestConditionFactory::ShipConstruction(count)
                | Kc3rdQuestConditionFactory::SlotItemConstruction(count)
                | Kc3rdQuestConditionFactory::SlotItemImprovement(count),
            )
            | Kc3rdQuestCondition::Scrap(
                Kc3rdQuestConditionScrap::AnyEquipment(count)
                | Kc3rdQuestConditionScrap::AnyShip(count),
            )
            | Kc3rdQuestCondition::Repair(count)
            | Kc3rdQuestCondition::Resupply(count)
            | Kc3rdQuestCondition::Sink(_, count) => *count == 0,
            // Composition validated separately via fleet check
            Kc3rdQuestCondition::Scrap(Kc3rdQuestConditionScrap::SpecificItems(items)) => {
                items.iter().all(|item| item.amount == 0)
            }
            Kc3rdQuestCondition::Composition(_) => false,
            Kc3rdQuestCondition::Sortie(s) => s.times == 0,
            Kc3rdQuestCondition::Exercise(e) => e.times == 0,
            Kc3rdQuestCondition::Expedition(exps) => exps.iter().all(|e| e.times == 0),
            Kc3rdQuestCondition::Modernization(m) => m.times == 0,
            // Consumption/ModelConversion are deducted at claim time, always "satisfied" for progress
            Kc3rdQuestCondition::Consumption(_) | Kc3rdQuestCondition::ModelConversion(_) => true,
        }
    }
}

/// Whether the condition at `idx` is met. A composition a sortie is fought in
/// was judged at each battle, so it is met once all its sorties are. Pairing is
/// read off the master, as the matcher reads it.
fn satisfied_at(
    conditions: &[Kc3rdQuestCondition],
    master: &[Kc3rdQuestCondition],
    idx: usize,
) -> bool {
    if composition_is_paired(master, idx) {
        return (0..master.len())
            .filter(|&sortie| paired_composition_index(master, sortie) == Some(idx))
            .all(|sortie| conditions.get(sortie).is_some_and(Kc3rdQuestCondition::is_satisfied));
    }
    conditions[idx].is_satisfied()
}

fn progress_from_ratio(completed: usize, total: usize) -> QuestProgressStatus {
    if total == 0 {
        return QuestProgressStatus::Completed;
    }
    let ratio = completed as f64 / total as f64;
    if ratio >= 1.0 {
        QuestProgressStatus::Completed
    } else if ratio >= 0.8 {
        QuestProgressStatus::Eighty
    } else if ratio >= 0.5 {
        QuestProgressStatus::Half
    } else {
        QuestProgressStatus::Empty
    }
}
