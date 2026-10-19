use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

// `docs/apilist.txt:3835` notes the client fixes the friendly formation to
// 第四警戒航行序列 (14) on these cells; the value is still read from the request
// rather than assumed.
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
    let resp = state.sortie_combined_ld_shooting(pid, params.order()).await?;

    Ok(KcApiResponse::success(&resp))
}
