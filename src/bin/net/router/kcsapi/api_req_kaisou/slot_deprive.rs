use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	// source ship id
	api_unset_ship: i64,
	// target ship id
	api_set_ship: i64,
	// source slot kind: 0: normal slot, 1: extended slot
	api_unset_slot_kind: i64,
	// target slot kind: 0: normal slot, 1: extended slot
	api_set_slot_kind: i64,
	// source slot index to deprive from
	api_unset_idx: i64,
	// target slot index to deprive to
	api_set_idx: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct ShipData {
	api_set_ship: KcApiShip,
	api_unset_ship: KcApiShip,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	api_ship_data: ShipData,

	#[serde(skip_serializing_if = "Option::is_none")]
	api_unset_list: Option<KcApiUnsetListElement>,

	#[serde(skip_serializing_if = "Option::is_none")]
	api_bauxite: Option<i64>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let from_ship_id = params.api_unset_ship;
	let to_ship_id = params.api_set_ship;
	let from_ex_slot = params.api_unset_slot_kind == 1;
	let to_ex_slot = params.api_set_slot_kind == 1;
	let from_slot_idx = params.api_unset_idx;
	let to_slot_idx = params.api_set_idx;

	let resp = state
		.slot_deprive(
			pid,
			&SlotDepriveParams {
				from_ship_id,
				to_ship_id,
				from_ex_slot,
				to_ex_slot,
				from_slot_idx,
				to_slot_idx,
			},
		)
		.await?;

	let api_unset_list = if let Some(unset_id_list) = resp.unset_id_list {
		Some(KcApiUnsetListElement {
			api_type_3_no: resp.unset_type3.unwrap(),
			api_slot_list: unset_id_list,
		})
	} else {
		None
	};

	let resp = Resp {
		api_ship_data: ShipData {
			api_set_ship: resp.to_ship.into(),
			api_unset_ship: resp.from_ship.into(),
		},
		api_unset_list,
		api_bauxite: None,
	};

	Ok(KcApiResponse::success(&resp))
}
