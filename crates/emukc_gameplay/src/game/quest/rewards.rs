use emukc_db::sea_orm::ConnectionTrait;
use emukc_model::codex::Codex;

use crate::err::GameplayError;

pub(super) async fn claim_quest_rewards<C>(
	c: &C,
	codex: &Codex,
	profile_id: i64,
	quest_id: i64,
	reward_choices: Option<&[i64]>,
) -> Result<(), GameplayError>
where
	C: ConnectionTrait,
{
	let quest_mst = codex
		.quest
		.get(&quest_id)
		.ok_or(GameplayError::EntryNotFound(format!("quest {quest_id} not found in codex")))?;
	todo!()
}
