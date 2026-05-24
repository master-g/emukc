use axum::{Extension, Json};
use emukc_internal::prelude::PayItemOps;
use serde::{Deserialize, Serialize};

use crate::net::{AppState, auth::GameSession};
use crate::state::State;

#[derive(Debug, Deserialize)]
pub(super) struct ConfirmQuery {
    payment_id: String,
    #[allow(dead_code)]
    st: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConfirmResponse {
    response_code: String,
    payment_id: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    response_code: String,
    msg: String,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    axum::extract::Query(query): axum::extract::Query<ConfirmQuery>,
) -> Json<serde_json::Value> {
    let state: &State = state.as_ref();

    let session_data = match state.payment_store.take(&query.payment_id) {
        Some(s) => s,
        None => {
            return Json(serde_json::json!(ErrorResponse {
                response_code: "ERROR".to_string(),
                msg: "payment session not found".to_string(),
            }));
        }
    };

    if session_data.profile_id != session.profile.id {
        return Json(serde_json::json!(ErrorResponse {
            response_code: "ERROR".to_string(),
            msg: "profile mismatch".to_string(),
        }));
    }

    if let Err(e) =
        state.add_pay_item(session.profile.id, session_data.sku_id, session_data.count).await
    {
        return Json(serde_json::json!(ErrorResponse {
            response_code: "ERROR".to_string(),
            msg: format!("failed to add item: {e}"),
        }));
    }

    Json(serde_json::json!(ConfirmResponse {
        response_code: "OK".to_string(),
        payment_id: query.payment_id,
    }))
}
