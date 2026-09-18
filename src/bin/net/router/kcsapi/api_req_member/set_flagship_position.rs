use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    api_position_id: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.update_flagship_position(pid, params.api_position_id).await?;

    Ok(KcApiResponse::empty())
}
