use axum::Form;

use crate::net::prelude::*;

use super::battle::Params;

// `docs/apilist.txt` lists no request body for this endpoint; per the file's own
// convention it inherits the single-fleet one at `docs/apilist.txt:2031`, so the
// client also sends api_recovery_type, api_supply_flag, api_ration_flag and
// api_smoke_flag. Only the last is used here.
pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_combined_battle_water(pid, params.order()).await?;

    Ok(KcApiResponse::success(&resp))
}
