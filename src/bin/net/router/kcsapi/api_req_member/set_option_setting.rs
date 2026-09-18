use axum::Form;

use crate::net::prelude::*;

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<KcApiOptionSetting>,
) -> KcApiResult {
    state.update_options_settings(pid, &params).await?;

    Ok(KcApiResponse::empty())
}
