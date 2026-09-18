use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    state.expand_repair_dock(pid).await?;

    Ok(KcApiResponse::empty())
}
