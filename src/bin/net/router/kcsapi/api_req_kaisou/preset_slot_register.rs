use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    api_preset_id: i64,
    api_ship_id: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.register_preset_slot(pid, params.api_preset_id, params.api_ship_id).await?;

    Ok(KcApiResponse::empty())
}
