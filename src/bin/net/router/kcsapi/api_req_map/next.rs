use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

use super::projection::project_next;

#[derive(Deserialize)]
pub(super) struct Params {
    #[serde(default)]
    #[allow(dead_code)]
    pub(super) api_recovery_type: i64,
    #[serde(default)]
    pub(super) api_cell_id: Option<i64>,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;
    let resp = state.next_sortie(pid, params.api_cell_id).await?;

    Ok(KcApiResponse::success(&project_next(resp)))
}
