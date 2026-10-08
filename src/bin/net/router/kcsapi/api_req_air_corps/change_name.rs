use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area the airbase belongs to.
    pub(super) api_area_id: i64,
    /// Airbase id within the area.
    pub(super) api_base_id: i64,
    /// The new name.
    pub(super) api_name: String,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.rename_airbase(pid, params.api_area_id, params.api_base_id, &params.api_name).await?;

    Ok(KcApiResponse::empty())
}
