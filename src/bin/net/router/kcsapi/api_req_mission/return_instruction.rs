use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

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
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    state.recall_expedition(pid, params.api_deck_id).await?;
    let fleet = state.get_fleet(pid, params.api_deck_id).await?;
    let deck: KcApiDeckPort = fleet.into();

    Ok(KcApiResponse::success(&Resp {
        api_mission: deck.api_mission,
    }))
}
