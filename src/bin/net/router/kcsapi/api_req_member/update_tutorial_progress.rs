use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    api_no: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.update_tutorial_progress(pid, params.api_no).await?;
    Ok(KcApiResponse::empty())
}
