use axum::{Router, routing::post};

mod next;
mod select_eventmap_rank;
mod start;

pub(super) fn router() -> Router {
	Router::new()
		.route("/next", post(next::handler))
		.route("/select_eventmap_rank", post(select_eventmap_rank::handler))
		.route("/start", post(start::handler))
}
