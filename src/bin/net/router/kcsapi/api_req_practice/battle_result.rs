use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let resp = state.practice_battle_result(pid).await?;

    Ok(KcApiResponse::success(&resp))
}
