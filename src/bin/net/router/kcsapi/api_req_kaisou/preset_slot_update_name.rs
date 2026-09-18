use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    api_preset_id: i64,
    api_name: String,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.update_preset_slot_name(pid, params.api_preset_id, &params.api_name).await?;

    Ok(KcApiResponse::empty())
}
