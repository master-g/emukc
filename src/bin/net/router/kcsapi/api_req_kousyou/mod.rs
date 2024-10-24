use axum::{routing::post, Router};

mod createitem;
mod createship;

pub(super) fn router() -> Router {
	Router::new()
		.route("/createitem", post(createitem::handler))
		.route("/createship", post(createship::handler))
}
