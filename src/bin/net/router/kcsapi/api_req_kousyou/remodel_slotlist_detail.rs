use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Recipe id, as handed out by `remodel_slotlist`.
    pub(super) api_id: i64,
    /// Instance id of the equipment to improve.
    pub(super) api_slot_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resp {
    api_req_buildkit: i64,
    api_req_remodelkit: i64,
    api_certain_buildkit: i64,
    api_certain_remodelkit: i64,
    api_req_slot_id: i64,
    api_req_slot_num: i64,
    /// The optional fields are omitted entirely when the recipe does not ask
    /// for a second equipment or for items — the client tests for their
    /// presence, not for a zero.
    #[serde(skip_serializing_if = "Option::is_none")]
    api_req_slot_id2: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_req_slot_num2: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_req_useitem_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_req_useitem_num: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_req_useitem_id2: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_req_useitem_num2: Option<i64>,
    api_change_flag: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let detail = state.remodel_slot_detail(pid, params.api_id, params.api_slot_id).await?;

    Ok(KcApiResponse::success(&Resp {
        api_req_buildkit: detail.req_buildkit,
        api_req_remodelkit: detail.req_remodelkit,
        api_certain_buildkit: detail.certain_buildkit,
        api_certain_remodelkit: detail.certain_remodelkit,
        api_req_slot_id: detail.req_slot_id,
        api_req_slot_num: detail.req_slot_num,
        api_req_slot_id2: detail.req_slot_id2,
        api_req_slot_num2: detail.req_slot_num2,
        api_req_useitem_id: detail.req_useitem_id,
        api_req_useitem_num: detail.req_useitem_num,
        api_req_useitem_id2: detail.req_useitem_id2,
        api_req_useitem_num2: detail.req_useitem_num2,
        api_change_flag: i64::from(detail.change_flag),
    }))
}
