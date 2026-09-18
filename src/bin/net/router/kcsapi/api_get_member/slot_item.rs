use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let slot_items = state.get_slot_items(pid).await?;
    Ok(KcApiResponse::success(&slot_items))
}
