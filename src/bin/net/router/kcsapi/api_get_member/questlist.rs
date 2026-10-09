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
struct Resp {
    api_count: i64,
    api_completed_kind: i64,
    api_list: Vec<KcApiQuestItem>,
    api_exec_count: i64,

    // never used
    api_exec_type: i64,
    // `api_c_list` is left out on purpose. The official server sends it so the
    // client can finish judging a conversion quest from the player's holdings;
    // this server judges the holdings itself and sends `api_state` 3, which the
    // client's own check leaves alone.
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
        api_voice_id: item.voice_id,
        api_get_material: item.reward_materials,
        api_select_rewards: item.select_rewards,
        api_bonus_flag: item.bonus_flag,
        api_progress_flag: item.progress_flag,
        api_invalid_flag: i64::from(item.invalid),
    }
}
