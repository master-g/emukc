use async_trait::async_trait;
use emukc_db::{
	entity::profile::{
		expedition,
		quest::{self, progress::Status},
	},
	sea_orm::{ActiveValue, IntoActiveModel, QueryOrder, TransactionTrait, entity::prelude::*},
};
use emukc_model::{
	codex::{Codex, query::FoundInCodex},
	kc2::KcApiQuestClearItemGet,
	thirdparty::{Kc3rdQuest, reward::get_quest_rewards},
};
use emukc_time::chrono;
use update::update_quests_impl;

use crate::{
	err::GameplayError,
	game::quest::{record::mark_quest_as_completed, rewards::claim_quest_rewards},
	gameplay::HasContext,
};

mod record;
mod rewards;
pub(crate) mod update;

/// A trait for quest related gameplay.
#[async_trait]
pub trait QuestOps {
	/// Get all quest records of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_quest_records(
		&self,
		profile_id: i64,
	) -> Result<Vec<quest::progress::Model>, GameplayError>;

	/// Add a quest to a profile.
	/// for debugging only
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `quest_id`: The quest ID.
	async fn quest_add(&self, profile_id: i64, quest_id: i64) -> Result<(), GameplayError>;

	/// Start a quest for a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `quest_id`: The quest ID.
	async fn quest_start(&self, profile_id: i64, quest_id: i64) -> Result<(), GameplayError>;

	/// Stop a quest for a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `quest_id`: The quest ID.
	async fn quest_stop(&self, profile_id: i64, quest_id: i64) -> Result<(), GameplayError>;

	/// Clear a quest and claim its reward for a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	/// - `quest_id`: The quest ID.
	/// - `reward_choices`: The reward choices, if any.
	async fn quest_clear_and_claim_reward(
		&self,
		profile_id: i64,
		quest_id: i64,
		reward_choices: Option<Vec<i64>>,
	) -> Result<KcApiQuestClearItemGet, GameplayError>;
}

async fn update_quest_status<C>(
	c: &C,
	profile_id: i64,
	quest_id: i64,
	status: Status,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let quest = quest::progress::Entity::find()
		.filter(quest::progress::Column::ProfileId.eq(profile_id))
		.filter(quest::progress::Column::QuestId.eq(quest_id))
		.one(c)
		.await?
		.ok_or(GameplayError::EntryNotFound(format!(
			"quest {quest_id} not found in profile {profile_id}"
		)))?;

	if quest.status == status {
		return Err(GameplayError::QuestStatusInvalid(format!(
			"quest {quest_id} in profile {profile_id} is already in status {status:?}"
		)));
	}

	let mut am = quest.into_active_model();
	am.status = ActiveValue::Set(status);

	am.update(c).await?;

	Ok(())
}

#[async_trait]
impl<T: HasContext + ?Sized> QuestOps for T {
	/// Get all quest records of a profile.
	///
	/// # Parameters
	///
	/// - `profile_id`: The profile ID.
	async fn get_quest_records(
		&self,
		profile_id: i64,
	) -> Result<Vec<quest::progress::Model>, GameplayError> {
		let codex = self.codex();
		let db = self.db();
		let mut tx = db.begin().await?;

		if update_quests_impl(&tx, codex, profile_id).await? {
			tx.commit().await?;

			tx = db.begin().await?;
		}

		Ok(quest::progress::Entity::find()
			.filter(quest::progress::Column::ProfileId.eq(profile_id))
			.order_by_asc(quest::progress::Column::QuestId)
			.all(&tx)
			.await?)
	}

	async fn quest_add(&self, profile_id: i64, quest_id: i64) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		let codex = self.codex();
		let quest_manifest = Kc3rdQuest::find_in_codex(codex, &quest_id)?;

		let (requirements, typ) = match &quest_manifest.requirements {
			emukc_model::thirdparty::Kc3rdQuestRequirement::And(kc3rd_quest_conditions) => {
				(kc3rd_quest_conditions, quest::progress::RequirementType::And)
			}
			emukc_model::thirdparty::Kc3rdQuestRequirement::OneOf(kc3rd_quest_conditions) => {
				(kc3rd_quest_conditions, quest::progress::RequirementType::OneOf)
			}
			emukc_model::thirdparty::Kc3rdQuestRequirement::Sequential(kc3rd_quest_conditions) => {
				(kc3rd_quest_conditions, quest::progress::RequirementType::Sequential)
			}
		};

		let am = quest::progress::ActiveModel {
			id: ActiveValue::NotSet,
			profile_id: ActiveValue::Set(profile_id),
			quest_id: ActiveValue::Set(quest_id),
			status: ActiveValue::Set(Status::Idle),
			progress: ActiveValue::Set(quest::progress::Progress::Completed),
			period: ActiveValue::Set(quest_manifest.period.into()),
			start_since: ActiveValue::Set(chrono::Utc::now()),
			requirements: ActiveValue::Set(serde_json::to_value(requirements).unwrap()),
			requirement_type: ActiveValue::Set(typ),
		};

		am.insert(&tx).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn quest_start(&self, profile_id: i64, quest_id: i64) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_quest_status(&tx, profile_id, quest_id, Status::Activated).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn quest_stop(&self, profile_id: i64, quest_id: i64) -> Result<(), GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		update_quest_status(&tx, profile_id, quest_id, Status::Idle).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn quest_clear_and_claim_reward(
		&self,
		profile_id: i64,
		quest_id: i64,
		reward_choices: Option<Vec<i64>>,
	) -> Result<KcApiQuestClearItemGet, GameplayError> {
		let db = self.db();
		let tx = db.begin().await?;

		// find the quest
		let quest = quest::progress::Entity::find()
			.filter(quest::progress::Column::ProfileId.eq(profile_id))
			.filter(quest::progress::Column::QuestId.eq(quest_id))
			.one(&tx)
			.await?
			.ok_or(GameplayError::EntryNotFound(format!(
				"quest {quest_id} not found in profile {profile_id}"
			)))?;

		// mark as completed
		mark_quest_as_completed(&tx, profile_id, quest_id, quest.period).await?;

		// remove quest progress
		{
			let mut am = quest.into_active_model();
			am.status = ActiveValue::Set(Status::Idle);
			// am.update(&tx).await?;
			am.delete(&tx).await?;
		}

		// reconstruct quest tree
		// this will be called by mainjs, but we do it here to ensure consistency
		let codex = self.codex();
		update_quests_impl(&tx, codex, profile_id).await?;

		// claim rewards
		claim_quest_rewards(&tx, codex, profile_id, quest_id, reward_choices.as_deref()).await?;

		// get rewards for kcs API response
		let resp = get_quest_rewards(codex, quest_id, reward_choices.as_deref())?;

		tx.commit().await?;

		Ok(resp)
	}
}

pub(super) async fn init<C>(c: &C, codex: &Codex, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	update_quests_impl(c, codex, profile_id).await?;

	Ok(())
}

pub(super) async fn wipe<C>(c: &C, profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	quest::oneshot::Entity::delete_many()
		.filter(expedition::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;
	quest::periodic::Entity::delete_many()
		.filter(expedition::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;
	quest::progress::Entity::delete_many()
		.filter(expedition::Column::ProfileId.eq(profile_id))
		.exec(c)
		.await?;

	Ok(())
}
