use axum::{Router, routing::post};

mod port;

pub(super) fn router() -> Router {
	Router::new().route("/port", post(port::handler))
}
