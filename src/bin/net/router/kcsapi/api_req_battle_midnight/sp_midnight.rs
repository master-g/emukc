use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    #[serde(default = "default_formation")]
    pub(super) api_formation: i64,
}

fn default_formation() -> i64 {
    1
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_sp_midnight_battle(pid, params.api_formation).await?;

    Ok(KcApiResponse::success(&resp))
}
