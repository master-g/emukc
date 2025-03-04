use axum::{Router, routing::post};

mod charge;

pub(super) fn router() -> Router {
	Router::new().route("/charge", post(charge::handler))
}
