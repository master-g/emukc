use axum::{Router, routing::post};

mod battle;
mod sp_midnight;

pub(super) fn router() -> Router {
	Router::new()
		.route("/battle", post(battle::handler))
		.route("/sp_midnight", post(sp_midnight::handler))
}
