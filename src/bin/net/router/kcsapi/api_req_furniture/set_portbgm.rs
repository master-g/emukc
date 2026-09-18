use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
    api_music_id: i64,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let codex = state.codex();
    let mst = codex.find::<KcApiMusicListElement>(&params.api_music_id)?;

    state.update_port_bgm(pid, mst.api_bgm_id).await?;

    Ok(KcApiResponse::empty())
}
