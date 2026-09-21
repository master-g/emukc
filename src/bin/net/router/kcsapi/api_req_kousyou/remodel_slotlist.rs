use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

/// One improvable equipment, as the arsenal's selection screen lists it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resp {
    api_id: i64,
    api_slot_id: i64,
    api_req_fuel: i64,
    api_req_bull: i64,
    api_req_steel: i64,
    api_req_bauxite: i64,
    api_req_buildkit: i64,
    api_req_remodelkit: i64,
    /// Always 0 here; the per-equipment cost comes from `remodel_slotlist_detail`.
    api_req_slot_id: i64,
    api_req_slot_num: i64,
    /// 0 = normal. 1 and 2 drive the 「特別改修」 speech bubble, which we do not emit.
    api_sp_type: i64,
}

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let entries = state.remodel_slot_list(pid).await?;

    let resp: Vec<Resp> = entries
        .into_iter()
        .map(|entry| Resp {
            api_id: entry.recipe_id,
            api_slot_id: entry.slot_item_id,
            api_req_fuel: entry.req_fuel,
            api_req_bull: entry.req_ammo,
            api_req_steel: entry.req_steel,
            api_req_bauxite: entry.req_bauxite,
            api_req_buildkit: entry.req_buildkit,
            api_req_remodelkit: entry.req_remodelkit,
            api_req_slot_id: 0,
            api_req_slot_num: 0,
            api_sp_type: 0,
        })
        .collect();

    Ok(KcApiResponse::success(&resp))
}
