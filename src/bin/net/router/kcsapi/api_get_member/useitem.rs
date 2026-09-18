use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let use_items = state.get_use_items(pid).await?;
    Ok(KcApiResponse::success(&use_items))
}
