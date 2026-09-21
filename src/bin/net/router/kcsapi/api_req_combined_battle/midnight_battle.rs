use crate::net::prelude::*;

// 連合艦隊 vs 通常艦隊 夜戦. Only 第2艦隊 fights, which the session decides from
// the ships' own deck tags — the same entry serves the single-fleet endpoint.
pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let resp = state.sortie_midnight_battle(pid).await?;

    Ok(KcApiResponse::success(&resp))
}
