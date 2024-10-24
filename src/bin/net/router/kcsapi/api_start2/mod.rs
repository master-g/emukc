use axum::{response::IntoResponse, routing::post, Router};

use crate::net::{resp::KcApiResponse, AppState};

pub(super) fn router() -> Router {
	Router::new().route("/getData", post(handler))
}

async fn handler(state: AppState) -> impl IntoResponse {
	KcApiResponse::success(&state.codex.manifest)
}
