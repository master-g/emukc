use crate::net::prelude::*;

// The client sends api_btime and api_l_value..api_l_value4 (client-side timing and
// layout hints); none of them is used here, so no form is extracted.
pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let resp = state.sortie_battle_result(pid).await?;

    Ok(KcApiResponse::success(&resp))
}
