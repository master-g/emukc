use crate::net::prelude::*;

pub(super) async fn handler(state: AppState, Pid(pid): Pid) -> KcApiResult {
    let rivals = state.get_practice_rivals(pid).await?;

    Ok(KcApiResponse::success(&KcApiPracticeResp {
        api_create_kind: rivals.cfg.generated_type as i64,
        api_selected_kind: rivals.cfg.selected_type as i64,
        api_entry_limit: rivals.entry_limit,
        api_list: rivals.rivals.into_iter().map(std::convert::Into::into).collect(),
    }))
}
