use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// 0 第一群, 1 第二群, 2 全体.
    pub(super) api_selected_kind: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resp {
    api_update_flag: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.change_practice_matching_kind(pid, params.api_selected_kind).await?;

    Ok(KcApiResponse::success(&Resp {
        api_update_flag: 1,
    }))
}
