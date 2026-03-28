use axum::{Router, routing::post};

mod battle;

pub(super) fn router() -> Router {
	Router::new().route("/battle", post(battle::handler))
}
