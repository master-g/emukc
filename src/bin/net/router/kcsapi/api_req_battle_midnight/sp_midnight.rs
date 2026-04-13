use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    #[serde(default = "default_formation")]
    pub(super) api_formation: i64,
}

fn default_formation() -> i64 {
    1
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;
    let resp = state.sortie_sp_midnight_battle(pid, params.api_formation).await?;

    Ok(KcApiResponse::success(&resp))
}
