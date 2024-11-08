use axum::{routing::post, Router};

mod can_preset_slot_select;
mod lock;
mod marriage;
mod open_exslot;

pub(super) fn router() -> Router {
	Router::new()
		.route("/can_preset_slot_select", post(can_preset_slot_select::handler))
		.route("/lock", post(lock::handler))
		.route("/marriage", post(marriage::handler))
		.route("open_exslot", post(open_exslot::handler))
}
