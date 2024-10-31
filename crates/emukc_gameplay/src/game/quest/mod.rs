use async_trait::async_trait;
use emukc_db::{
	entity::profile::{expedition, quest},
	sea_orm::{entity::prelude::*, TransactionTrait},
};
use update::update_quests_impl;

use crate::{err::GameplayError, gameplay::HasContext};

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
		let tx = db.begin().await?;

		update_quests_impl(&tx, codex, profile_id).await?;

		todo!()
	}
}

pub(super) async fn init<C>(_c: &C, _profile_id: i64) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
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
