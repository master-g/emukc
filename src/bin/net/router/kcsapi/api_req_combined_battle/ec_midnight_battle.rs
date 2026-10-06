use crate::net::prelude::*;

// 通常艦隊 vs 敵連合艦隊 夜戦. Which enemy deck fights is the session's call; the
// same entry serves every night battle that follows a day one.
pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let resp = state.sortie_midnight_battle(pid).await?;

    Ok(KcApiResponse::success(&resp))
}
