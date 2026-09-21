use crate::net::prelude::*;

// Same shape as `api_req_sortie/battleresult`; the combined-only fields
// (`api_mvp_combined` and the two `_combined` experience arrays) come from the
// pending result snapshot, which already knows both decks.
pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let resp = state.sortie_battle_result(pid).await?;

    Ok(KcApiResponse::success(&resp))
}
