use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

use super::projection::project_next;

#[derive(Deserialize)]
pub(super) struct Params {
    #[serde(default)]
    #[expect(dead_code)]
    pub(super) api_recovery_type: i64,
    #[serde(default)]
    pub(super) api_cell_id: Option<i64>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.next_sortie(pid, params.api_cell_id).await?;

    Ok(KcApiResponse::success(&project_next(resp)))
}
