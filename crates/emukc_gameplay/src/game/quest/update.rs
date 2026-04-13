use emukc_db::{
    entity::profile::quest::{
        ShouldReset, oneshot,
        periodic::{self},
        progress,
    },
    sea_orm::{ActiveValue, IntoActiveModel, entity::prelude::*},
};
use emukc_model::{
    codex::Codex,
    prelude::{Kc3rdQuest, Kc3rdQuestCondition, Kc3rdQuestRequirement},
    profile::quest::QuestProgressStatus,
    thirdparty::QuestActionEvent,
};
use emukc_time::chrono;

use crate::err::GameplayError;

pub(crate) async fn update_quests_impl<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
) -> Result<bool, GameplayError>
where
    C: ConnectionTrait,
{
    let mut should_commit = false;
    let mut completed_quest_id: Vec<i64> = Vec::new();

    // one-shot quests
    let oneshot_quests =
        oneshot::Entity::find().filter(oneshot::Column::ProfileId.eq(profile_id)).all(c).await?;

    for quest in oneshot_quests {
        completed_quest_id.push(quest.quest_id);
    }

    // reset periodical quests first
    let periodic_quests =
        periodic::Entity::find().filter(periodic::Column::ProfileId.eq(profile_id)).all(c).await?;

    for quest in periodic_quests {
        if quest.should_reset() {
            should_commit = true;
            quest.delete(c).await?;
        } else {
            completed_quest_id.push(quest.quest_id);
        }
    }

    // in progress quests
    let in_progress_quests =
        progress::Entity::find().filter(progress::Column::ProfileId.eq(profile_id)).all(c).await?;

    let mut in_progress_quest_id: Vec<i64> = Vec::new();

    for quest in in_progress_quests.iter() {
        if quest.should_reset() {
            should_commit = true;
            progress::Entity::delete_by_id(quest.id).exec(c).await?;
        } else {
            // recalculate progress only for activated quests
            if quest.status == progress::Status::Activated {
                let mst = codex.find::<Kc3rdQuest>(&quest.quest_id).unwrap();
                should_commit |= recalculate_quest_progress(c, mst, quest).await?;
            }
            in_progress_quest_id.push(quest.quest_id);
        }
    }

    // reconstruct quest tree
    let new_quests =
        reconstruct_quest_tree(codex, profile_id, &completed_quest_id, &in_progress_quest_id)
            .await?;

    if !new_quests.is_empty() {
        should_commit = true;
        // insert new quests
        for quest in new_quests {
            quest.insert(c).await?;
        }
    }

    Ok(should_commit)
}

async fn recalculate_quest_progress<C>(
    c: &C,
    mst: &Kc3rdQuest,
    model: &progress::Model,
) -> Result<bool, GameplayError>
where
    C: ConnectionTrait,
{
    let mut changed = false;
    // current requirements
    let conditions: Vec<Kc3rdQuestCondition> = serde_json::from_value(model.requirements.clone())?;
    let requirements = match model.requirement_type {
        progress::RequirementType::And => Kc3rdQuestRequirement::And(conditions),
        progress::RequirementType::OneOf => Kc3rdQuestRequirement::OneOf(conditions),
        progress::RequirementType::Sequential => Kc3rdQuestRequirement::Sequential(conditions),
    };

    // calculate progress
    let progress = requirements.calculate_progress(&mst.requirements);

    let progress = match (progress, model.status) {
        // if the quest is completed but not activated, set it to 80%
        (QuestProgressStatus::Completed, progress::Status::Idle) => QuestProgressStatus::Eighty,
        _ => progress,
    };
    let progress: progress::Progress = progress.into();
    // update progress if necessary
    if model.progress != progress {
        changed = true;

        let mut am = model.clone().into_active_model();
        am.progress = ActiveValue::Set(progress);

        am.update(c).await?;
    }

    Ok(changed)
}

async fn reconstruct_quest_tree(
    codex: &Codex,
    profile_id: i64,
    completed_quest_id: &[i64],
    in_progress_quest_id: &[i64],
) -> Result<Vec<progress::ActiveModel>, GameplayError> {
    let new_quests: Vec<progress::ActiveModel> = codex
        .quest
        .iter()
        .filter_map(|(id, quest)| {
            // check if the quest is already completed or in progress
            if completed_quest_id.contains(id) || in_progress_quest_id.contains(id) {
                return None;
            }

            // check if the quest is available
            if quest.prerequisite.iter().any(|id| !completed_quest_id.contains(id)) {
                return None;
            }

            let (requirement_type, conditions) = match &quest.requirements {
                Kc3rdQuestRequirement::And(vec) => (progress::RequirementType::And, vec),
                Kc3rdQuestRequirement::OneOf(vec) => (progress::RequirementType::OneOf, vec),
                Kc3rdQuestRequirement::Sequential(vec) => {
                    (progress::RequirementType::Sequential, vec)
                }
            };

            Some(progress::ActiveModel {
                id: ActiveValue::NotSet,
                profile_id: ActiveValue::Set(profile_id),
                quest_id: ActiveValue::Set(*id),
                status: ActiveValue::Set(progress::Status::Idle),
                progress: ActiveValue::Set(progress::Progress::Empty),
                period: ActiveValue::Set(quest.period.into()),
                start_since: ActiveValue::Set(chrono::Utc::now()),
                requirement_type: ActiveValue::Set(requirement_type),
                requirements: ActiveValue::Set(serde_json::to_value(conditions).unwrap()),
            })
        })
        .collect();

    Ok(new_quests)
}

/// Update quest progress for a game action
pub(crate) async fn update_quest_progress_for_action<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
    event: &QuestActionEvent,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    let include_idle_exercise = matches!(event, QuestActionEvent::ExerciseBattleCompleted { .. });

    let mut query = progress::Entity::find().filter(progress::Column::ProfileId.eq(profile_id));
    if include_idle_exercise {
        query = query.filter(
            progress::Column::Status.is_in([progress::Status::Activated, progress::Status::Idle]),
        );
    } else {
        query = query.filter(progress::Column::Status.eq(progress::Status::Activated));
    }

    let quests = query.all(c).await?;

    // 2. For each quest, check if event matches and update
    for quest in quests {
        let mut conditions: Vec<Kc3rdQuestCondition> =
            serde_json::from_value(quest.requirements.clone())?;
        let mst = codex.find::<Kc3rdQuest>(&quest.quest_id)?;
        let master_conditions = match (&quest.requirement_type, &mst.requirements) {
            (progress::RequirementType::And, Kc3rdQuestRequirement::And(conditions)) => {
                Some(conditions.as_slice())
            }
            (progress::RequirementType::OneOf, Kc3rdQuestRequirement::OneOf(conditions)) => {
                Some(conditions.as_slice())
            }
            (
                progress::RequirementType::Sequential,
                Kc3rdQuestRequirement::Sequential(conditions),
            ) => Some(conditions.as_slice()),
            _ => None,
        };
        if include_idle_exercise
            && quest.status == progress::Status::Idle
            && !master_conditions
                .is_some_and(|conditions| conditions.iter().any(is_exercise_condition))
        {
            continue;
        }

        let mut updated = false;
        for (idx, condition) in conditions.iter_mut().enumerate() {
            let master_condition = master_conditions.and_then(|conditions| conditions.get(idx));
            if condition.apply_event_with_context(event, master_condition, Some(codex)) {
                updated = true;
            }
        }

        if updated {
            let new_progress = progress_after_event(
                quest.status,
                &conditions,
                &mst.requirements,
                include_idle_exercise,
            );

            // Update database
            let mut am = quest.into_active_model();
            am.requirements = ActiveValue::Set(serde_json::to_value(&conditions)?);
            am.progress = ActiveValue::Set(new_progress.into());
            am.update(c).await?;
        }
    }

    Ok(())
}

fn is_exercise_condition(condition: &Kc3rdQuestCondition) -> bool {
    matches!(condition, Kc3rdQuestCondition::Exercise(_))
}

fn progress_after_event(
    status: progress::Status,
    current_conditions: &[Kc3rdQuestCondition],
    master_requirements: &Kc3rdQuestRequirement,
    include_idle_exercise: bool,
) -> emukc_model::profile::quest::QuestProgressStatus {
    if include_idle_exercise {
        if let Some(exercise_progress) =
            calculate_exercise_progress(current_conditions, master_requirements)
        {
            if status == progress::Status::Idle
                && exercise_progress == QuestProgressStatus::Completed
            {
                QuestProgressStatus::Eighty
            } else {
                exercise_progress
            }
        } else {
            let progress = calculate_progress_from_current(current_conditions, master_requirements);
            if status == progress::Status::Idle && progress == QuestProgressStatus::Completed {
                QuestProgressStatus::Eighty
            } else {
                progress
            }
        }
    } else {
        calculate_progress_from_current(current_conditions, master_requirements)
    }
}

fn calculate_progress_from_current(
    current_conditions: &[Kc3rdQuestCondition],
    master_requirements: &Kc3rdQuestRequirement,
) -> QuestProgressStatus {
    match master_requirements {
        Kc3rdQuestRequirement::And(_) => Kc3rdQuestRequirement::And(current_conditions.to_vec())
            .calculate_progress(master_requirements),
        Kc3rdQuestRequirement::OneOf(_) => {
            Kc3rdQuestRequirement::OneOf(current_conditions.to_vec())
                .calculate_progress(master_requirements)
        }
        Kc3rdQuestRequirement::Sequential(_) => {
            Kc3rdQuestRequirement::Sequential(current_conditions.to_vec())
                .calculate_progress(master_requirements)
        }
    }
}

fn calculate_exercise_progress(
    current_conditions: &[Kc3rdQuestCondition],
    master_requirements: &Kc3rdQuestRequirement,
) -> Option<QuestProgressStatus> {
    let master_conditions = match master_requirements {
        Kc3rdQuestRequirement::And(conditions)
        | Kc3rdQuestRequirement::OneOf(conditions)
        | Kc3rdQuestRequirement::Sequential(conditions) => conditions,
    };

    let mut total = 0_i64;
    let mut remaining = 0_i64;

    for (current, master) in current_conditions.iter().zip(master_conditions.iter()) {
        let (Kc3rdQuestCondition::Exercise(current), Kc3rdQuestCondition::Exercise(master)) =
            (current, master)
        else {
            continue;
        };
        total += master.times.max(0);
        remaining += current.times.max(0);
    }

    if total <= 0 {
        return None;
    }

    let completed = total - remaining;
    let ratio = completed as f64 / total as f64;
    Some(if ratio >= 1.0 {
        QuestProgressStatus::Completed
    } else if ratio >= 0.8 {
        QuestProgressStatus::Eighty
    } else if ratio >= 0.5 {
        QuestProgressStatus::Half
    } else if ratio > 0.0 {
        QuestProgressStatus::Half
    } else {
        QuestProgressStatus::Empty
    })
}

/// Validate composition quests when quest list is retrieved.
///
/// Composition conditions are special: they are not tracked via counters but
/// evaluated in real-time against the current fleet state. This function checks
/// all activated quests that contain Composition conditions, and updates their
/// stored progress accordingly (both setting and rolling back).
pub(crate) async fn validate_composition_quests<C>(
    c: &C,
    codex: &Codex,
    profile_id: i64,
) -> Result<(), GameplayError>
where
    C: ConnectionTrait,
{
    use crate::game::fleet::get_fleets_impl;
    use crate::game::ship::get_ships_impl;
    use emukc_model::thirdparty::composition::{ShipInstance, validate_composition};

    // Query all Activated quests
    let activated_quests = progress::Entity::find()
        .filter(progress::Column::ProfileId.eq(profile_id))
        .filter(progress::Column::Status.eq(progress::Status::Activated))
        .all(c)
        .await?;

    // Load fleets and ships
    let fleets = get_fleets_impl(c, profile_id).await?;
    let (ships, _) = get_ships_impl(c, profile_id).await?;

    // Convert to ShipInstance
    let ship_instances: Vec<ShipInstance> = ships
        .iter()
        .map(|s| ShipInstance {
            id: s.id,
            mst_id: s.mst_id,
            level: s.level,
        })
        .collect();

    // Convert fleet models to Fleet
    let fleet_profiles: Vec<emukc_model::profile::fleet::Fleet> =
        fleets.iter().map(|f| f.clone().into()).collect();

    // Check each quest
    for quest in activated_quests {
        let stored_conditions: Vec<Kc3rdQuestCondition> =
            serde_json::from_value(quest.requirements.clone())?;
        let master_conditions = if let Some(master_quest) = codex.quest.get(&quest.quest_id) {
            match &master_quest.requirements {
                Kc3rdQuestRequirement::And(conditions)
                | Kc3rdQuestRequirement::OneOf(conditions)
                | Kc3rdQuestRequirement::Sequential(conditions) => conditions.as_slice(),
            }
        } else {
            stored_conditions.as_slice()
        };

        // Only process quests that have at least one Composition condition
        let has_composition =
            master_conditions.iter().any(|c| matches!(c, Kc3rdQuestCondition::Composition(_)));
        if !has_composition {
            continue;
        }

        // Evaluate composition conditions against current fleet
        let composition_satisfied = match quest.requirement_type {
            progress::RequirementType::And => {
                // All composition conditions must be satisfied
                master_conditions.iter().all(|cond| {
                    if let Kc3rdQuestCondition::Composition(comp_cond) = cond {
                        fleet_profiles
                            .iter()
                            .find(|f| f.index == comp_cond.fleet_id || comp_cond.fleet_id == 0)
                            .is_some_and(|fleet| {
                                validate_composition(fleet, &ship_instances, comp_cond, codex)
                            })
                    } else {
                        // Non-composition conditions are evaluated elsewhere
                        true
                    }
                })
            }
            progress::RequirementType::OneOf => {
                // Any composition condition being satisfied is enough
                master_conditions.iter().any(|cond| {
                    if let Kc3rdQuestCondition::Composition(comp_cond) = cond {
                        fleet_profiles
                            .iter()
                            .find(|f| f.index == comp_cond.fleet_id || comp_cond.fleet_id == 0)
                            .is_some_and(|fleet| {
                                validate_composition(fleet, &ship_instances, comp_cond, codex)
                            })
                    } else {
                        false
                    }
                })
            }
            progress::RequirementType::Sequential => {
                // For sequential, check composition conditions in order
                master_conditions.iter().all(|cond| {
                    if let Kc3rdQuestCondition::Composition(comp_cond) = cond {
                        fleet_profiles
                            .iter()
                            .find(|f| f.index == comp_cond.fleet_id || comp_cond.fleet_id == 0)
                            .is_some_and(|fleet| {
                                validate_composition(fleet, &ship_instances, comp_cond, codex)
                            })
                    } else {
                        true
                    }
                })
            }
        };

        // For And/Sequential quests with mixed conditions, also check non-composition via is_satisfied
        let all_non_composition_satisfied = stored_conditions.iter().all(|cond| {
            if matches!(cond, Kc3rdQuestCondition::Composition(_)) {
                true // skip composition here, handled above
            } else {
                // Use the master quest to check counter-based progress
                cond.is_satisfied()
            }
        });

        let fully_satisfied = composition_satisfied && all_non_composition_satisfied;

        // Update progress: set Completed or rollback
        if fully_satisfied && quest.progress != progress::Progress::Completed {
            let mut am = quest.into_active_model();
            am.progress = ActiveValue::Set(progress::Progress::Completed);
            am.update(c).await?;
        } else if !composition_satisfied && quest.progress == progress::Progress::Completed {
            // Rollback: composition no longer satisfied
            let mut am = quest.into_active_model();
            am.progress = ActiveValue::Set(progress::Progress::Empty);
            am.update(c).await?;
        }
    }

    Ok(())
}
