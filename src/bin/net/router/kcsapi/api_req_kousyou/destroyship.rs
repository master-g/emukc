use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// ship id to destroy
	api_ship_id: i64,

	/// 0: keep equipment, 1: destroy equipment
	api_slot_dest_flag: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_material: Vec<i64>,
	api_unset_list: KcApiUnsetSlot,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let codex = state.codex();

	state.destroy_ship(pid, params.api_ship_id, params.api_slot_dest_flag == 0).await?;

	let materials = state.get_materials(pid).await?;
	let unset_slots = state.get_unset_slot_items(pid).await?;
	let api_unset_list: KcApiUnsetSlot = codex.convert_unused_slot_items_to_api(&unset_slots)?;

	Ok(KcApiResponse::success(&Resp {
		api_material: vec![materials.fuel, materials.ammo, materials.steel, materials.bauxite],
		api_unset_list,
	}))
}
