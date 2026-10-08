use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area the airbase belongs to.
    pub(super) api_area_id: i64,
    /// Airbase id within the area.
    pub(super) api_base_id: i64,
    /// Squadron slots to resupply, comma separated.
    #[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_ivec")]
    pub(super) api_squadron_id: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resp {
    api_after_fuel: i64,
    api_after_bauxite: i64,
    /// Absent from `docs/apilist.txt`; the decoded client reads it
    /// (`AirUnitSupplyAPI`) and hands it to `updateSquadronData`.
    api_distance: KcApiDistance,
    /// The squadrons the call named.
    api_plane_info: Vec<KcApiPlaneInfo>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let result = state
        .supply_airbase(pid, params.api_area_id, params.api_base_id, &params.api_squadron_id)
        .await?;

    let (api_base, api_bonus) = result.distance;

    Ok(KcApiResponse::success(&Resp {
        api_after_fuel: result.after_fuel,
        api_after_bauxite: result.after_bauxite,
        api_distance: KcApiDistance {
            api_base,
            api_bonus,
        },
        api_plane_info: result.updated.into_iter().map(std::convert::Into::into).collect(),
    }))
}
