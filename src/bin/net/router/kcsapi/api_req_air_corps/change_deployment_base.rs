use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area both airbases belong to.
    pub(super) api_area_id: i64,
    /// Airbase the squadron moves to.
    pub(super) api_base_id: i64,
    /// Airbase the squadron comes from.
    pub(super) api_base_id_src: i64,
    /// Destination squadron slot, 1-based.
    ///
    /// Absent from `docs/apilist.txt`, which lists only the three ids — the
    /// decoded client posts it (`AirUnitChangeDeployBaseAPI`).
    pub(super) api_squadron_id: i64,
    /// Equipment instance id being moved.
    pub(super) api_item_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resp {
    /// Both airbases, whole.
    api_base_items: Vec<KcApiAirBase>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let airbases = state
        .change_deployment_base(
            pid,
            params.api_area_id,
            params.api_base_id,
            params.api_base_id_src,
            params.api_squadron_id,
            params.api_item_id,
        )
        .await?;

    Ok(KcApiResponse::success(&Resp {
        api_base_items: airbases.into_iter().map(std::convert::Into::into).collect(),
    }))
}
