use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    api_deck_id: i64,
}

#[derive(Serialize)]
struct Resp {
    api_mission: [i64; 4],
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;
    state.recall_expedition(pid, params.api_deck_id).await?;
    let fleet = state.get_fleet(pid, params.api_deck_id).await?;
    let deck: KcApiDeckPort = fleet.into();

    Ok(KcApiResponse::success(&Resp {
        api_mission: deck.api_mission,
    }))
}
