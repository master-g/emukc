use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    pub(super) api_formation: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_combined_airbattle(pid, params.api_formation).await?;

    Ok(KcApiResponse::success(&resp))
}
