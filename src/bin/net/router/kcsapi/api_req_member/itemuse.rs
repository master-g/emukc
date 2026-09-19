use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    api_useitem_id: i64,

    // 1: medal for blueprint
    // 61: raw food materials for rice balls
    api_exchange_type: Option<i64>,

    // 0: response.api_caution_flag will be 1 if the material will be capped by limit.
    // 1: response.api_caution_flag will be 0 if the material will be capped by limit.
    #[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_flag")]
    api_force_flag: bool,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state
        .consume_use_item(
            pid,
            params.api_useitem_id,
            params.api_exchange_type.unwrap_or(0),
            params.api_force_flag,
        )
        .await?;

    Ok(KcApiResponse::success(&resp))
}
