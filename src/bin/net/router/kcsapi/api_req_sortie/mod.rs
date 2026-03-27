use axum::{Router, routing::post};

mod battle;
mod battleresult;

pub(super) fn router() -> Router {
	Router::new()
		.route("/battle", post(battle::handler))
		.route("/battleresult", post(battleresult::handler))
}
