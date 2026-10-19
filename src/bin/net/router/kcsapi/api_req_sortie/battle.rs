use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

// The client also sends api_recovery_type, api_supply_flag and api_ration_flag,
// which are not used here.
#[derive(Deserialize)]
pub(super) struct Params {
    pub(super) api_formation: i64,
    /// 1 when the player asks for a smoke screen; absent otherwise.
    #[serde(default)]
    pub(super) api_smoke_flag: i64,
}

impl Params {
    pub(super) fn order(&self) -> SortieOrder {
        SortieOrder {
            formation_id: self.api_formation,
            smoke: self.api_smoke_flag == 1,
        }
    }
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_battle(pid, params.order()).await?;

    Ok(KcApiResponse::success(&resp))
}
