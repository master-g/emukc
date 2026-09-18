use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let preset_slot = state.get_preset_slots(pid).await?;
    let resp: KcApiPresetSlot = preset_slot.into();

    Ok(KcApiResponse::success(&resp))
}
