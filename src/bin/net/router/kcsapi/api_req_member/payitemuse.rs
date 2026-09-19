use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    api_payitem_id: i64,

    // 0: response.api_caution_flag will be 1 if the material will be capped by limit.
    // 1: response.api_caution_flag will be 0 if the material will be capped by limit.
    #[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_flag")]
    api_force_flag: bool,
}

#[derive(Serialize, Default)]
struct Resp {
    // 0: will not show caution dialog, 1: will show caution dialog
    api_caution_flag: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let caution = state.consume_pay_item(pid, params.api_payitem_id, params.api_force_flag).await?;

    Ok(KcApiResponse::success(&Resp {
        api_caution_flag: if caution {
            1
        } else {
            0
        },
    }))
}
