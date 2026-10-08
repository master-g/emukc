use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    pub(super) api_preset_no: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.toggle_preset_deck_lock(pid, params.api_preset_no).await?;

    // The client flips its own copy of the flag and reads nothing here.
    Ok(KcApiResponse::empty())
}
