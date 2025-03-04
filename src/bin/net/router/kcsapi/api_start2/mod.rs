use axum::{Router, response::IntoResponse, routing::post};

use crate::net::{AppState, resp::KcApiResponse};

mod get_option_setting;

pub(super) fn router() -> Router {
	Router::new()
		.route("/getData", post(handler))
		.route("/get_option_setting", post(get_option_setting::handler))
}

async fn handler(state: AppState) -> impl IntoResponse {
	KcApiResponse::success(&state.codex.manifest)
}
