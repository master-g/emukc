use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let docks = state.get_kdocks(pid).await?;
    let docks: Vec<KcApiKDock> = docks.into_iter().map(std::convert::Into::into).collect();

    Ok(KcApiResponse::success(&docks))
}
