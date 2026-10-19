use axum::Form;

use crate::net::prelude::*;

use super::battle::Params;

// The client also sends api_recovery_type, api_supply_flag and api_ration_flag,
// which are not used here.
pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_ld_shooting(pid, params.order()).await?;

    Ok(KcApiResponse::success(&resp))
}
