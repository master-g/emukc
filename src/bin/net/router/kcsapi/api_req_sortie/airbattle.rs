use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    pub(super) api_formation: i64,
    #[serde(default)]
    pub(super) api_recovery_type: i64,
    #[serde(default)]
    pub(super) api_supply_flag: Option<i64>,
    #[serde(default)]
    pub(super) api_ration_flag: Option<i64>,
    #[serde(default)]
    pub(super) api_smoke_flag: Option<i64>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let _ = (
        params.api_recovery_type,
        params.api_supply_flag,
        params.api_ration_flag,
        params.api_smoke_flag,
    );
    let resp = state.sortie_airbattle(pid, params.api_formation).await?;

    Ok(KcApiResponse::success(&resp))
}
