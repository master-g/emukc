use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

// `docs/apilist.txt:3159` notes the friendly formation is a string here where
// the day endpoints send a number. Form bodies are strings either way, so
// `serde_urlencoded` parses both into the same i64 and the note costs nothing.
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
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let resp = state.sortie_combined_sp_midnight_battle(pid, params.api_formation).await?;

    Ok(KcApiResponse::success(&resp))
}
