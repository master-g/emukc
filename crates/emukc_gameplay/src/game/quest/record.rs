use emukc_db::{
	entity::profile::quest::{Period, oneshot, periodic},
	sea_orm::{ActiveValue, entity::prelude::*},
};
use emukc_time::chrono;

use crate::err::GameplayError;

pub(super) async fn mark_quest_as_completed<C>(
	c: &C,
	profile_id: i64,
	quest_id: i64,
	period: Period,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	if period == Period::Oneshot {
		let old_entry = oneshot::Entity::find()
			.filter(oneshot::Column::ProfileId.eq(profile_id))
			.filter(oneshot::Column::QuestId.eq(quest_id))
			.one(c)
			.await?;
		if old_entry.is_none() {
			let am = oneshot::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				quest_id: ActiveValue::Set(quest_id),
				complete_time: ActiveValue::Set(chrono::Utc::now()),
			};
			am.insert(c).await?;
		}
	} else {
		let old_entry = periodic::Entity::find()
			.filter(periodic::Column::ProfileId.eq(profile_id))
			.filter(periodic::Column::QuestId.eq(quest_id))
			.one(c)
			.await?;
		if old_entry.is_none() {
			let am = periodic::ActiveModel {
				id: ActiveValue::NotSet,
				profile_id: ActiveValue::Set(profile_id),
				quest_id: ActiveValue::Set(quest_id),
				complete_time: ActiveValue::Set(chrono::Utc::now()),
				period: ActiveValue::Set(period),
			};

			am.insert(c).await?;
		}
	}

	Ok(())
}
