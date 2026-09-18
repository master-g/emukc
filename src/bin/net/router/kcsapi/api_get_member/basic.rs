use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let (_, basic) = state.get_user_basic(pid).await?;
    Ok(KcApiResponse::success(&basic))
}
