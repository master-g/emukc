use axum::Form;
use emukc::model::profile::fleet::Fleet;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
    api_deck_id: i64,
    api_preset_no: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let fleet = state.apply_preset_deck(pid, params.api_deck_id, params.api_preset_no).await?;
    let fleet: Fleet = fleet.into();
    let resp: KcApiDeckPort = fleet.into();

    Ok(KcApiResponse::success(&resp))
}
