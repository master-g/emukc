mod can_preset_slot_select;

use axum::{routing::post, Router};

pub(super) fn router() -> Router {
	Router::new().route("/can_preset_slot_select", post(can_preset_slot_select::handler))
}
