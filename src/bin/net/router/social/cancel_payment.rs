use axum::Json;
use serde::{Deserialize, Serialize};

use crate::net::AppState;
use crate::state::State;

#[derive(Debug, Deserialize)]
pub(super) struct CancelQuery {
    payment_id: String,
    #[allow(dead_code)]
    st: Option<String>,
}

#[derive(Debug, Serialize)]
struct CancelResponse {
    response_code: String,
    payment_id: String,
}

pub(super) async fn handler(
    state: AppState,
    axum::extract::Query(query): axum::extract::Query<CancelQuery>,
) -> Json<serde_json::Value> {
    let state: &State = state.as_ref();
    let payment_id = query.payment_id.clone();
    state.payment_store.take(&query.payment_id);

    Json(serde_json::json!(CancelResponse {
        response_code: "CANCEL".to_string(),
        payment_id,
    }))
}
