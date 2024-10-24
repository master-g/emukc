mod firstship;
mod nickname;

use axum::{routing::post, Router};

pub(super) fn router() -> Router {
	Router::new()
		.route("/nickname", post(nickname::handler))
		.route("/firstship", post(firstship::handler))
}
