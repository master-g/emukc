use axum::{routing::post, Router};

mod can_preset_slot_select;
mod lock;

pub(super) fn router() -> Router {
	Router::new()
		.route("/can_preset_slot_select", post(can_preset_slot_select::handler))
		.route("/lock", post(lock::handler))
}
