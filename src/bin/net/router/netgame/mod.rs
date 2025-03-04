mod gadgets;
mod purchase;

use axum::{
	Router,
	routing::{get, post},
};

pub(super) fn router() -> Router {
	Router::new().nest(
		"/social",
		Router::new()
			.route("/-/gadgets/", post(gadgets::handler))
			.nest("/application", Router::new().route("/purchase", get(purchase::handler))),
	)
}
