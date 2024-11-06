use axum::{routing::post, Router};

mod charge;

pub(super) fn router() -> Router {
	Router::new().route("/charge", post(charge::handler))
}
