use axum::{Router, routing::post};
use serde::Serialize;

use crate::net::prelude::*;

pub(super) fn router() -> Router {
    Router::new().route("/paycheck", post(handler))
}

#[derive(Serialize)]
struct PaycheckResp {
    api_check_value: i64,
}

pub(super) async fn handler(_state: AppState) -> KcApiResult {
    Ok(KcApiResponse::success(&PaycheckResp {
        api_check_value: 1,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::router::kcsapi::test_utils::{app_state, new_test_context};

    #[tokio::test]
    async fn paycheck_returns_success_with_check_value_1() {
        let context = new_test_context().await;
        let resp = handler(app_state(&context.state)).await.unwrap();
        let data = resp.api_data.unwrap();
        assert_eq!(data["api_check_value"], 1);
    }
}
