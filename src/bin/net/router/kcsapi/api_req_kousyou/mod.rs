use axum::{Router, routing::post};

mod createitem;
mod createship;
mod createship_speedchange;
mod destroyitem2;
mod destroyship;
mod getship;
mod open_new_dock;
mod preset_dev_items_delete;
mod preset_dev_items_expand;
mod preset_dev_items_register;
mod preset_dev_items_update_name;

pub(super) fn router() -> Router {
	Router::new()
		.route("/createitem", post(createitem::handler))
		.route("/createship", post(createship::handler))
		.route("/createship_speedchange", post(createship_speedchange::handler))
		.route("/destroyitem2", post(destroyitem2::handler))
		.route("/destroyship", post(destroyship::handler))
		.route("/getship", post(getship::handler))
		.route("/open_new_dock", post(open_new_dock::handler))
		.route("/preset_dev_items_register", post(preset_dev_items_register::handler))
		.route("/preset_dev_items_delete", post(preset_dev_items_delete::handler))
		.route("/preset_dev_items_update_name", post(preset_dev_items_update_name::handler))
		.route("/preset_dev_items_expand", post(preset_dev_items_expand::handler))
}
