use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    /// 0: All
    /// 1: Daily
    /// 2: Weekly
    /// 3: Monthly
    /// 4: One-time
    /// 5: Others
    /// 9: Activated
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
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let view = state.quest_list_view(pid, params.api_tab_id).await?;

    Ok(KcApiResponse::success(&project(view)))
}

fn project(view: QuestListView) -> Resp {
    let api_list: Vec<KcApiQuestItem> = view.items.into_iter().map(project_item).collect();

    Resp {
        api_count: api_list.len() as i64,
        api_completed_kind: view.completed_kind,
        api_list,
        api_exec_count: view.exec_count,
        api_exec_type: 0,
        api_c_list: vec![], // TODO(#0): we are not there yet
    }
}

fn project_item(item: QuestListItem) -> KcApiQuestItem {
    KcApiQuestItem {
        api_no: item.no,
        api_category: item.category,
        api_type: item.quest_type,
        api_label_type: item.label_type,
        api_state: item.state,
        api_title: item.title,
        api_detail: item.detail,
        api_lost_badges: item.lost_badges,
        api_voice_id: 0, // TODO(#0): voice_id is missing now
        api_get_material: item.reward_materials,
        api_select_rewards: item.select_rewards,
        api_bonus_flag: item.bonus_flag,
        api_progress_flag: item.progress_flag,
        api_invalid_flag: 0, // TODO(#0): invalid_flag is missing now, (e.g: plane convert)
    }
}
