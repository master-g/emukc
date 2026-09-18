use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    api_slotitem_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
    api_locked: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let item = state.toggle_slot_item_locked(pid, params.api_slotitem_id).await?;

    Ok(KcApiResponse::success(&Resp {
        api_locked: item.api_locked,
    }))
}
