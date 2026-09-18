use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct NicknameParams {
    api_nickname: String,
    api_nickname_id: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<NicknameParams>,
) -> KcApiResult {
    state.update_user_nickname(pid, &params.api_nickname).await?;

    let (_, basic) = state.get_user_basic(pid).await?;

    Ok(KcApiResponse::success(&basic))
}
