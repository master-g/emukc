use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let preset_dev_items = state.get_preset_dev_items(pid).await?;
    let resp: KcApiPresetDevItem = preset_dev_items.into();

    Ok(KcApiResponse::success(&resp))
}
