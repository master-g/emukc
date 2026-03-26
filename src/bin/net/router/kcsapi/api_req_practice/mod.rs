use axum::{Router, routing::post};

mod battle;
mod battle_result;

pub(super) fn router() -> Router {
	Router::new()
		.route("/battle", post(battle::handler))
		.route("/battle_result", post(battle_result::handler))
}
