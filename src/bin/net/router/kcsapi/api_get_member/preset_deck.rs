use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let preset_decks = state.get_preset_decks(pid).await?;
    let resp: KcApiPresetDeck = preset_decks.into();

    Ok(KcApiResponse::success(&resp))
}
