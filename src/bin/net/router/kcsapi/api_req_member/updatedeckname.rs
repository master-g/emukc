use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    api_deck_id: i64,
    api_name_id: String,
    api_name: String,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    debug!(
        "update_deck_name: pid={}, deck_id={}, name_id={}, name={}",
        pid, params.api_deck_id, params.api_name_id, params.api_name
    );

    state.update_deck_name(pid, params.api_deck_id, &params.api_name).await?;

    Ok(KcApiResponse::empty())
}
