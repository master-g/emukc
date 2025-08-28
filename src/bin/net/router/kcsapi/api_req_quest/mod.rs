use axum::{Router, routing::post};

mod clearitemget;
mod start;
mod stop;

pub(super) fn router() -> Router {
	Router::new()
		.route("/start", post(start::handler))
		.route("/stop", post(stop::handler))
		.route("/clearitemget", post(clearitemget::handler))
}
