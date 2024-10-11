use axum::{routing::post, Router};

mod get_worldinfo;
mod register;

pub(super) fn router() -> Router {
	Router::new()
		.route("/get_worldinfo", post(get_worldinfo::handler))
		.route("/register", post(register::handler))
}
