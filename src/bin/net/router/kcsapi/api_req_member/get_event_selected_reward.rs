use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;
//
#[derive(Deserialize)]
pub(super) struct Params {
    api_selected_dict: i64,
}

pub(super) async fn handler(
    _state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    debug!("get_event_selected_reward: pid={}, selected_dict={}", pid, params.api_selected_dict);

    Ok(KcApiResponse::empty())
}
