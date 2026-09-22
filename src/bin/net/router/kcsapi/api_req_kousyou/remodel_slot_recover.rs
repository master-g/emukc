use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Arsenal menu id, which is the recipe id `remodel_slotlist` handed out.
    pub(super) api_menu_id: i64,
    /// Instance id of the equipment to reset.
    pub(super) api_slot_id: i64,
    /// 開発資材 the player put in; the client offers 1, 2 or 3.
    pub(super) api_dev_num: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resp {
    api_recover_flag: i64,
    /// Absent on a failed attempt; carries the same instance id on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    api_after_slot: Option<KcApiSlotItem>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let result = state
        .remodel_slot_recover(pid, params.api_menu_id, params.api_slot_id, params.api_dev_num)
        .await?;

    Ok(KcApiResponse::success(&Resp {
        api_recover_flag: i64::from(result.success),
        api_after_slot: result.after_slot,
    }))
}
