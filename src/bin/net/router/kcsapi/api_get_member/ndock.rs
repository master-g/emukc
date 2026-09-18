use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let docks = state.get_ndocks(pid).await?;
    let docks: Vec<KcApiNDock> = docks.into_iter().map(std::convert::Into::into).collect();

    Ok(KcApiResponse::success(&docks))
}
