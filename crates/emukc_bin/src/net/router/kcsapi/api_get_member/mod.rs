mod require_info;
use axum::{routing::post, Router};

pub(super) fn router() -> Router {
	Router::new().route("/require_info", post(require_info::handler))
}
