use axum::{Router, routing::post};

mod airbattle;
mod battle;
mod battleresult;
mod goback_port;

pub(super) fn router() -> Router {
	Router::new()
		.route("/airbattle", post(airbattle::handler))
		.route("/battle", post(battle::handler))
		.route("/battleresult", post(battleresult::handler))
		.route("/goback_port", post(goback_port::handler))
}
