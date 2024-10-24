use axum::{routing::post, Router};

mod firstship;
mod nickname;

pub(super) fn router() -> Router {
	Router::new()
		.route("/nickname", post(nickname::handler))
		.route("/firstship", post(firstship::handler))
}
