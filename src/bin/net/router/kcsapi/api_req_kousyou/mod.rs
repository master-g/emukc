use axum::{routing::post, Router};

mod createitem;
mod createship;
mod createship_speedchange;
mod destroyitem2;
mod destroyship;
mod getship;
mod open_new_dock;

pub(super) fn router() -> Router {
	Router::new()
		.route("/createitem", post(createitem::handler))
		.route("/createship", post(createship::handler))
		.route("/createship_speedchange", post(createship_speedchange::handler))
		.route("/destroyitem2", post(destroyitem2::handler))
		.route("/destroyship", post(destroyship::handler))
		.route("/getship", post(getship::handler))
		.route("/open_new_dock", post(open_new_dock::handler))
}
