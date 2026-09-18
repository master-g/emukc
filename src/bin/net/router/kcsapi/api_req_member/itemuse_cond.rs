use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    // 1 = mamiya, 2: irako, 3: mamiya+irako
    api_use_type: i64,

    // deck id
    api_deck_id: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.consume_cond_use_item(pid, params.api_deck_id, params.api_use_type).await?;

    Ok(KcApiResponse::empty())
}
