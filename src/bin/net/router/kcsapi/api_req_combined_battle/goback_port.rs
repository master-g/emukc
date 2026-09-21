use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let resp = state.sortie_goback_port(pid).await?;

    Ok(KcApiResponse::success(&resp))
}
