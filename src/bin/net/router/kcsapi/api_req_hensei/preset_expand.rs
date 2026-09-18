use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    state.expand_preset_deck_capacity(pid).await?;

    Ok(KcApiResponse::empty())
}
