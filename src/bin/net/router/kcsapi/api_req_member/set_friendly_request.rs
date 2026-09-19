use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;
//
#[derive(Deserialize)]
pub(super) struct Params {
    // 0: denied, 1: approved
    #[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_flag")]
    api_request_flag: bool,

    // 0: default, 1: powerful
    api_request_type: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    debug!(
        "set_friendly_request: pid={}, request_flag={}, request_type={}",
        pid, params.api_request_flag, params.api_request_type
    );

    state
        .update_friendly_fleet_settings(pid, params.api_request_flag, params.api_request_type)
        .await?;

    Ok(KcApiResponse::empty())
}
