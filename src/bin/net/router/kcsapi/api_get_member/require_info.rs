use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
struct UserBasic {
    api_member_id: i64,
    api_firstflag: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
    api_basic: UserBasic,
    api_extra_supply: [i64; 2],
    api_furniture: Vec<KcApiFurniture>,
    api_kdock: Vec<KcApiKDock>,
    api_oss_setting: KcApiOssSetting,
    api_position_id: i64,
    api_skin_id: i64,
    api_slot_item: Vec<KcApiSlotItem>,
    api_unsetslot: BTreeMap<String, Vec<i64>>,
    api_useitem: Vec<KcApiUserItem>,
}

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let view = state.require_info_view(pid).await?;
    Ok(KcApiResponse::success(&project(view)))
}

fn project(view: RequireInfoView) -> Resp {
    Resp {
        api_basic: UserBasic {
            api_member_id: view.member_id,
            api_firstflag: view.firstflag,
        },
        api_extra_supply: view.extra_supply,
        api_furniture: view.furnitures,
        api_kdock: view.kdocks.into_iter().map(std::convert::Into::into).collect(),
        api_oss_setting: view.oss_settings,
        api_position_id: view.position_id,
        api_skin_id: view.skin_id,
        api_slot_item: view.slot_items,
        api_unsetslot: view.unset_slots,
        api_useitem: view.use_items,
    }
}
