use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area that gets another air corps.
    pub(super) api_area_id: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let airbase: KcApiAirBase = state.expand_airbase(pid, params.api_area_id).await?.into();

    // The client takes element 0 and adds it to the ones it already has.
    Ok(KcApiResponse::success(&[airbase]))
}
