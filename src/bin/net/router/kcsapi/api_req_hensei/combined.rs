use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    /// 0: disband, 1=機動部隊, 2=水上部隊, 3=輸送部隊
    api_combined_type: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
    /// 0: disband, 1=combined
    api_combined: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let api_combined = state.set_combined_type(pid, params.api_combined_type).await?;

    Ok(KcApiResponse::success(&Resp {
        api_combined,
    }))
}
