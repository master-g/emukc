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
    api_ship_id: i64,
    api_slot_pos: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
    api_onslot_max: [i64; 5],
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;

    let onslot_max = state.expand_hangar_slot(pid, params.api_ship_id, params.api_slot_pos).await?;

    let resp = Resp {
        api_onslot_max: onslot_max,
    };

    Ok(KcApiResponse::success(&resp))
}
