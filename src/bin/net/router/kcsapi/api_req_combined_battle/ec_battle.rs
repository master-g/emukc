use axum::Form;

use crate::net::prelude::*;

use super::battle::Params;

// 通常艦隊 vs 敵連合艦隊 昼戦. The client comes here instead of
// `api_req_sortie/battle` when the cell's event kind is 5 (`map_info.isVS12()`).
pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_ec_battle(pid, params.api_formation).await?;

    Ok(KcApiResponse::success(&resp))
}
