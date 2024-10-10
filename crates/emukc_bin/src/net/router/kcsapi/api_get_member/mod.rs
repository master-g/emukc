use axum::{routing::post, Router};

mod basic;
mod require_info;

pub(super) fn router() -> Router {
	Router::new()
		.route("/require_info", post(require_info::handler))
		.route("/basic", post(basic::handler))
}
