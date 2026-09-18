use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let furnitures = state.get_furnitures(pid).await?;

    Ok(KcApiResponse::success(&furnitures))
}
