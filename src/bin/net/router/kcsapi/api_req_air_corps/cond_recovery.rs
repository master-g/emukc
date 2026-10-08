use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area the airbase belongs to.
    pub(super) api_area_id: i64,
    /// Airbase id within the area.
    pub(super) api_base_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resp {
    api_distance: KcApiDistance,
    /// The squadrons of the airbase that fly.
    api_plane_info: Vec<KcApiPlaneInfo>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let result =
        state.recover_airbase_condition(pid, params.api_area_id, params.api_base_id).await?;

    let (api_base, api_bonus) = result.distance;

    Ok(KcApiResponse::success(&Resp {
        api_distance: KcApiDistance {
            api_base,
            api_bonus,
        },
        api_plane_info: result.updated.into_iter().map(std::convert::Into::into).collect(),
    }))
}
