use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    /// should be 1
    api_highspeed: i64,
    /// construction dock id
    api_kdock_id: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.speed_up_ship_construction(pid, params.api_kdock_id).await?;

    Ok(KcApiResponse::empty())
}
