use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    api_ndock_id: i64,
    api_ship_id: i64,
    api_highspeed: i64,
}

#[derive(Serialize)]
struct Resp {
    api_material: Vec<i64>,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;

    let material = state
        .ndock_start_repair(pid, params.api_ndock_id, params.api_ship_id, params.api_highspeed == 1)
        .await?;

    if let Some(material) = material {
        let api_material: Vec<KcApiMaterialElement> = material.into();
        let api_material: Vec<i64> = api_material.into_iter().map(|v| v.api_value).collect();
        Ok(KcApiResponse::success(&Resp {
            api_material,
        }))
    } else {
        Ok(KcApiResponse::empty())
    }
}
