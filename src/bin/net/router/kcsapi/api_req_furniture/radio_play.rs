use crate::net::prelude::*;

/// This API is the handler for `/api_req_furniture/radio_play`.
/// By looking into `main.js`, this API is called when player click on some furniture (e.g. 318, 319)
/// which will play a hiss sound, and if the port's bgm is not default(101), then this API will be called
pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    state.update_port_bgm(pid, 101).await?;

    Ok(KcApiResponse::empty())
}
