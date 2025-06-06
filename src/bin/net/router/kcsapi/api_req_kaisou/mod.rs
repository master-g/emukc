use axum::{Router, routing::post};

mod can_preset_slot_select;
mod lock;
mod marriage;
mod open_exslot;
mod powerup;
mod preset_slot_delete;
mod preset_slot_expand;
mod preset_slot_register;
mod preset_slot_select;
mod preset_slot_update_exslot_flag;
mod preset_slot_update_lock;
mod preset_slot_update_name;
mod remodeling;
mod slot_deprive;
mod slot_exchange_index;
mod slotset;
mod slotset_ex;
mod unsetslot_all;

pub(super) fn router() -> Router {
	Router::new()
		.route("/can_preset_slot_select", post(can_preset_slot_select::handler))
		.route("/lock", post(lock::handler))
		.route("/marriage", post(marriage::handler))
		.route("/open_exslot", post(open_exslot::handler))
		.route("/powerup", post(powerup::handler))
		.route("/preset_slot_delete", post(preset_slot_delete::handler))
		.route("/preset_slot_expand", post(preset_slot_expand::handler))
		.route("/preset_slot_register", post(preset_slot_register::handler))
		.route("/preset_slot_select", post(preset_slot_select::handler))
		.route("/preset_slot_update_exslot_flag", post(preset_slot_update_exslot_flag::handler))
		.route("/preset_slot_update_lock", post(preset_slot_update_lock::handler))
		.route("/preset_slot_update_name", post(preset_slot_update_name::handler))
		.route("/remodeling", post(remodeling::handler))
		.route("/slot_deprive", post(slot_deprive::handler))
		.route("/slot_exchange_index", post(slot_exchange_index::handler))
		.route("/slotset", post(slotset::handler))
		.route("/slotset_ex", post(slotset_ex::handler))
		.route("/unsetslot_all", post(unsetslot_all::handler))
}
