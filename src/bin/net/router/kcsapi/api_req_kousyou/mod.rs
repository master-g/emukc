use axum::{routing::post, Router};

mod createitem;

pub(super) fn router() -> Router {
	Router::new().route("/createitem", post(createitem::handler))
}
