use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area whose 整備Lv goes up by one.
    pub(super) api_area_id: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.expand_airbase_maintenance(pid, params.api_area_id).await?;

    // The client raises its own copy of the level; there is nothing to send.
    Ok(KcApiResponse::empty())
}
