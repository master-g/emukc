use axum::{routing::post, Router};

mod open_new_dock;
mod speedchange;
mod start;

pub(super) fn router() -> Router {
	Router::new()
		.route("/open_new_dock", post(open_new_dock::handler))
		.route("/speedchange", post(speedchange::handler))
		.route("/start", post(start::handler))
}
