use axum::{routing::post, Router};

mod port;

pub(super) fn router() -> Router {
	Router::new().route("/port", post(port::handler))
}
