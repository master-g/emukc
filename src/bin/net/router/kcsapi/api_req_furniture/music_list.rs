use crate::net::prelude::*;

pub(super) async fn handler(state: AppState) -> KcApiResult {
    let codex = state.codex();
    let music_list = &codex.music_list;

    Ok(KcApiResponse::success(music_list))
}
