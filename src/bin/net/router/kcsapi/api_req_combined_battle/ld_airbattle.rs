use axum::Form;

use crate::net::prelude::*;

use super::battle::Params;

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_combined_ld_airbattle(pid, params.order()).await?;

    Ok(KcApiResponse::success(&resp))
}
