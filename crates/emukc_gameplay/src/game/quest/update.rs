use emukc_db::{
	entity::profile::quest::{
		self,
		periodic::{self},
	},
	sea_orm::entity::prelude::*,
};
use emukc_model::codex::Codex;

use crate::err::GameplayError;

pub(crate) async fn update_quests_impl<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	// reset periodical quests first
	let periodic_quests = periodic::Entity::find()
		.filter(quest::periodic::Column::ProfileId.eq(profile_id))
		.all(c)
		.await?;

	for quest in periodic_quests {
		if quest.should_reset() {
			todo!()
		}
	}

	todo!()
}
