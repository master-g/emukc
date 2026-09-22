use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area the airbase belongs to.
    pub(super) api_area_id: i64,
    /// Airbase id within the area.
    pub(super) api_base_id: i64,
    /// Squadron slot, 1-based.
    pub(super) api_squadron_id: i64,
    /// Equipment instance id, or `-1` to clear the slot.
    pub(super) api_item_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resp {
    api_distance: KcApiDistance,
    /// Only the slots this call changed.
    api_plane_info: Vec<KcApiPlaneInfo>,
    /// Present only when the call spent bauxite.
    #[serde(skip_serializing_if = "Option::is_none")]
    api_after_bauxite: Option<i64>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let result = state
        .set_airbase_plane(
            pid,
            params.api_area_id,
            params.api_base_id,
            params.api_squadron_id,
            params.api_item_id,
        )
        .await?;

    let (api_base, api_bonus) = result.distance;

    Ok(KcApiResponse::success(&Resp {
        api_distance: KcApiDistance {
            api_base,
            api_bonus,
        },
        api_plane_info: result.updated.into_iter().map(std::convert::Into::into).collect(),
        api_after_bauxite: result.after_bauxite,
    }))
}
