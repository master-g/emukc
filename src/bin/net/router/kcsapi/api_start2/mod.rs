use axum::{response::IntoResponse, routing::post, Router};

use crate::net::{resp::KcApiResponse, AppState};

mod get_option_setting;

pub(super) fn router() -> Router {
	Router::new()
		.route("/getData", post(handler))
		.route("/get_option_setting", post(get_option_setting::handler))
}

async fn handler(state: AppState) -> impl IntoResponse {
	KcApiResponse::success(&state.codex.manifest)
}
