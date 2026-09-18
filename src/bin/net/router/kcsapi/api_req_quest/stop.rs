use axum::Form;

use crate::net::prelude::*;

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<super::start::Params>,
) -> KcApiResult {
    state.quest_stop(pid, params.api_quest_id).await?;

    Ok(KcApiResponse::empty())
}
