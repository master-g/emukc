use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize, Debug)]
pub(super) struct Params {
    api_ndock_id: i64,
}

#[derive(Serialize)]
struct Resp {
    api_material: Vec<i64>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.speed_up_ship_repairation(pid, params.api_ndock_id).await?;

    let materials = state.get_materials(pid).await?;
    let api_material = materials.into_array().to_vec();

    Ok(KcApiResponse::success(&Resp {
        api_material,
    }))
}
