use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// The preset that was dragged.
    pub(super) api_preset_from: i64,
    /// The preset it was dropped on.
    pub(super) api_preset_to: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.exchange_preset_decks(pid, params.api_preset_from, params.api_preset_to).await?;

    // The client exchanges the two itself and reads nothing here.
    Ok(KcApiResponse::empty())
}
