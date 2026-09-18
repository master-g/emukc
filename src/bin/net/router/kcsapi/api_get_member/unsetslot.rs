use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let codex = state.codex();

    let unset_slots = state.get_unset_slot_items(pid).await?;
    let resp: KcApiUnsetSlot = codex.convert_unused_slot_items_to_api(&unset_slots)?;

    Ok(KcApiResponse::success(&resp))
}
