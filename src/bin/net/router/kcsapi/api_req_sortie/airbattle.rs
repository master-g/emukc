use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

// The client also sends api_recovery_type, api_supply_flag, api_ration_flag and
// api_smoke_flag; none of them is used here.
#[derive(Deserialize)]
pub(super) struct Params {
    pub(super) api_formation: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_airbattle(pid, params.api_formation).await?;

    Ok(KcApiResponse::success(&resp))
}
