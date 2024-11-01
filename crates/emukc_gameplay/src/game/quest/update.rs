use emukc_db::{
	entity::profile::quest::{
		oneshot,
		periodic::{self},
		progress, ShouldReset,
	},
	sea_orm::{entity::prelude::*, ActiveValue, IntoActiveModel},
};
use emukc_model::{
	codex::Codex,
	prelude::{Kc3rdQuest, Kc3rdQuestCondition, Kc3rdQuestRequirement},
	profile::quest::QuestProgressStatus,
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
			// recalculate progress
			let mst = codex.find::<Kc3rdQuest>(&quest.quest_id).unwrap();
			should_commit = recalculate_quest_progress(c, mst, quest).await?;
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
