use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
    /// Area the airbases belong to.
    pub(super) api_area_id: i64,
    /// Airbase ids, comma separated.
    #[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_ivec")]
    pub(super) api_base_id: Vec<i64>,
    /// One order per airbase, comma separated: 0 待機, 1 出撃, 2 防空, 3 退避, 4 休息.
    #[serde(deserialize_with = "crate::net::router::kcsapi::form_utils::deserialize_form_ivec")]
    pub(super) api_action_kind: Vec<i64>,
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    if params.api_base_id.len() != params.api_action_kind.len() {
        return Err(GameplayError::WrongType(format!(
            "{} airbases but {} orders",
            params.api_base_id.len(),
            params.api_action_kind.len()
        ))
        .into());
    }
    let orders = params.api_base_id.into_iter().zip(params.api_action_kind).collect::<Vec<_>>();

    state.set_airbase_actions(pid, params.api_area_id, &orders).await?;

    Ok(KcApiResponse::empty())
}
