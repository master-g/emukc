use axum::Form;

use crate::net::prelude::*;

use super::battle::Params;

// `docs/apilist.txt:3835` notes the client fixes the friendly formation to
// 第四警戒航行序列 (14) on these cells; the value is still read from the request
// rather than assumed.
pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_combined_ld_shooting(pid, params.order()).await?;

    Ok(KcApiResponse::success(&resp))
}
