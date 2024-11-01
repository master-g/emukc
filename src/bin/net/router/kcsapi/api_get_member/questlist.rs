use axum::{Extension, Form};
use emukc::db::entity::profile::quest::progress;
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	api_tab_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CListItem {
	api_no: i64,
	api_progress_flag: i64,
	api_stage: i64,

	// 1: Completed
	api_c_flag: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_count: i64,
	api_completed_kind: i64,
	api_list: Vec<KcApiQuestItem>,
	api_exec_count: i64,

	// never used
	api_exec_type: i64,

	// those factory conversion quests will have this when in progress or completed
	#[serde(skip_serializing_if = "Vec::is_empty")]
	api_c_list: Vec<CListItem>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let codex = state.codex();
	let pid = session.profile.id;

	let quests = state.get_quest_records(pid).await?;

	let mut api_completed_kind = 0;
	let mut api_exec_count = 0;

	let api_list: Vec<KcApiQuestItem> = quests
		.iter()
		.filter_map(|model| {
			if params.api_tab_id == 0 && model.status != progress::Status::Activated {
				return None;
			}

			let mst = codex.find::<Kc3rdQuest>(&model.quest_id).ok()?;

			if params.api_tab_id > 0 && mst.label_type != params.api_tab_id {
				return None;
			}

			if model.status == progress::Status::Activated {
				api_exec_count += 1;
				if model.progress == progress::Progress::Completed {
					api_completed_kind = 1;
				}
			}

			Some(KcApiQuestItem {
				api_no: mst.api_no,
				api_category: mst.category as i64,
				api_type: match mst.period {
					Kc3rdQuestPeriod::Oneshot => KcApiQuestType::Oneshot as i64,
					Kc3rdQuestPeriod::Daily
					| Kc3rdQuestPeriod::Daily3rd7th0th
					| Kc3rdQuestPeriod::Daily2nd8th => KcApiQuestType::Daily as i64,
					Kc3rdQuestPeriod::Weekly => KcApiQuestType::Weekly as i64,
					Kc3rdQuestPeriod::Monthly => KcApiQuestType::Monthly as i64,
					Kc3rdQuestPeriod::Quarterly | Kc3rdQuestPeriod::Annual => {
						KcApiQuestType::Other as i64
					}
				},
				api_label_type: mst.label_type,
				api_state: if model.status == progress::Status::Idle {
					1
				} else if model.progress == progress::Progress::Completed {
					3
				} else {
					2
				},
				api_title: mst.name.clone(),
				api_detail: mst.detail.clone(),
				api_lost_badges: mst.requirements.lost_badges(),
				api_voice_id: 0, // TODO: voice_id is missing now
				api_get_material: vec![
					mst.reward_fuel,
					mst.reward_ammo,
					mst.reward_steel,
					mst.reward_bauxite,
				],
				api_select_rewards: mst.to_api_reward_selection(),
				api_bonus_flag: mst.bonus_flag(),
				api_progress_flag: if model.progress == progress::Progress::Completed {
					0
				} else {
					model.progress as i64
				},
				api_invalid_flag: 0, // TODO: invalid_flag is missing now, (e.g: plane convert)
			})
		})
		.collect();

	Ok(KcApiResponse::success(&Resp {
		api_count: api_list.len() as i64,
		api_completed_kind,
		api_list,
		api_exec_count,
		api_exec_type: 0,
		api_c_list: vec![], // TODO: we are not there yet
	}))
}
