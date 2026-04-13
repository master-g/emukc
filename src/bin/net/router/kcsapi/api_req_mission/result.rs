use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    api_deck_id: i64,
}

#[derive(Serialize)]
struct RespItem {
    api_useitem_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_useitem_name: Option<String>,
    api_useitem_count: i64,
}

#[derive(Serialize)]
struct Resp {
    api_ship_id: Vec<i64>,
    api_clear_result: i64,
    api_get_exp: i64,
    api_member_lv: i64,
    api_member_exp: i64,
    api_get_ship_exp: Vec<i64>,
    api_get_exp_lvup: Vec<[i64; 2]>,
    api_maparea_name: String,
    api_detail: String,
    api_quest_name: String,
    api_quest_level: i64,
    api_get_material: [i64; 4],
    api_useitem_flag: [i64; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    api_get_item1: Option<RespItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_get_item2: Option<RespItem>,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;
    let result = state.complete_expedition(pid, params.api_deck_id).await?;

    let mut api_ship_id = result.ship_ids.iter().rev().copied().collect::<Vec<_>>();
    api_ship_id.push(-1);

    let material = result.resource_reward.unwrap_or([-1; 4]);

    let mut api_useitem_flag = [0; 2];
    let mut api_items = [None, None];
    for (idx, reward) in result.item_rewards.iter().enumerate() {
        let Some(reward) = reward.as_ref() else {
            continue;
        };
        let (flag, item) = to_api_item(reward);
        api_useitem_flag[idx] = flag;
        api_items[idx] = Some(item);
    }

    Ok(KcApiResponse::success(&Resp {
        api_ship_id,
        api_clear_result: result.result as i64,
        api_get_exp: result.admiral_exp,
        api_member_lv: result.member_lv,
        api_member_exp: result.member_exp,
        api_get_ship_exp: result.ship_exp,
        api_get_exp_lvup: result.ship_exp_after,
        api_maparea_name: result.maparea_name,
        api_detail: result.detail,
        api_quest_name: result.quest_name,
        api_quest_level: result.quest_level,
        api_get_material: material,
        api_useitem_flag,
        api_get_item1: api_items[0].take(),
        api_get_item2: api_items[1].take(),
    }))
}

fn to_api_item(reward: &ExpeditionItemReward) -> (i64, RespItem) {
    match KcUseItemType::n(reward.item_id) {
        Some(KcUseItemType::Bucket) => (1, hidden_item(reward.count)),
        Some(KcUseItemType::Torch) => (2, hidden_item(reward.count)),
        Some(KcUseItemType::DevMaterial) => (3, hidden_item(reward.count)),
        Some(
            KcUseItemType::FCoinBox200
            | KcUseItemType::FCoinBox400
            | KcUseItemType::FCoinBox700
            | KcUseItemType::FCoin,
        ) => (5, hidden_item(reward.count)),
        _ => (
            4,
            RespItem {
                api_useitem_id: reward.item_id,
                api_useitem_name: Some(reward.name.clone()),
                api_useitem_count: reward.count,
            },
        ),
    }
}

fn hidden_item(count: i64) -> RespItem {
    RespItem {
        api_useitem_id: -1,
        api_useitem_name: None,
        api_useitem_count: count,
    }
}
