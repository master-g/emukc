use axum::{Router, routing::post};

mod change;
mod combined;
mod lock;
mod preset_delete;
mod preset_expand;
mod preset_register;
mod preset_select;

pub(super) fn router() -> Router {
	Router::new()
		.route("/change", post(change::handler))
		.route("/combined", post(combined::handler))
		.route("/lock", post(lock::handler))
		.route("/preset_delete", post(preset_delete::handler))
		.route("/preset_expand", post(preset_expand::handler))
		.route("/preset_register", post(preset_register::handler))
		.route("/preset_select", post(preset_select::handler))
}
