use anyhow::Result;
use clap::Args;

use crate::{cfg::AppConfig, state::State};
use emukc_internal::prelude::QuestOps;

/// Bootstrap command arguments
#[derive(Args, Debug)]
pub struct AddQuestArgs {
	#[arg(help = "profile id")]
	#[arg(long)]
	profile_id: i64,

	#[arg(help = "quest ids", value_name = "QUEST_ID")]
	quest_ids: Vec<i64>,
}

pub async fn exec(args: &AddQuestArgs, _cfg: &AppConfig, state: &State) -> Result<()> {
	for quest_id in &args.quest_ids {
		state.quest_add(args.profile_id, *quest_id).await?;
	}

	Ok(())
}
