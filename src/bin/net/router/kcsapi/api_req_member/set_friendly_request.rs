use axum::Form;
use emukc::prelude::SettingsOps;
use serde::Deserialize;

use crate::net::prelude::*;
//
#[derive(Deserialize)]
pub(super) struct Params {
    // 0: denied, 1: approved
    api_request_flag: i64,

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
        .update_friendly_fleet_settings(pid, params.api_request_flag == 1, params.api_request_type)
        .await?;

    Ok(KcApiResponse::empty())
}
