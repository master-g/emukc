use axum::{routing::post, Router};

mod basic;
mod require_info;
mod ship2;
mod ship3;
mod slot_item;

pub(super) fn router() -> Router {
	Router::new()
		.route("/basic", post(basic::handler))
		.route("/require_info", post(require_info::handler))
		.route("/ship2", post(ship2::handler))
		.route("/ship3", post(ship3::handler))
		.route("/slot_item", post(slot_item::handler))
}
