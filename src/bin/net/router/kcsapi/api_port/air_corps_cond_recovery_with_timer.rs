use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area the airbase belongs to.
    api_area_id: i64,
    /// Airbase id within the area.
    api_base_id: i64,
}

/// The client polls this for squadrons that recovered with time. With nothing
/// recovered upstream answers without `api_data`, which is all this can say
/// until something tires a squadron.
pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.check_airbase(pid, params.api_area_id, params.api_base_id).await?;

    Ok(KcApiResponse::empty())
}
