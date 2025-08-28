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

	#[arg(help = "quest id")]
	#[arg(long)]
	quest_id: i64,
}

pub async fn exec(args: &AddQuestArgs, _cfg: &AppConfig, state: &State) -> Result<()> {
	state.quest_add(args.profile_id, args.quest_id).await?;

	Ok(())
}
