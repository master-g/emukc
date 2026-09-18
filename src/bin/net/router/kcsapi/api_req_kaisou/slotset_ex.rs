use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
    api_id: i64,
    api_item_id: i64,
}

pub(super) async fn handler(state: AppState, Form(params): Form<Params>) -> KcApiResult {
    state.set_exslot_item(params.api_id, params.api_item_id).await?;

    Ok(KcApiResponse::empty())
}
