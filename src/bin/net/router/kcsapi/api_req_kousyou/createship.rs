use axum::{Extension, Form};
use rand::{rngs::SmallRng, seq::IndexedRandom, SeedableRng};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	auth::GameSession,
	err::ApiError,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// fuel
	api_item1: i64,
	/// ammo
	api_item2: i64,
	/// steel
	api_item3: i64,
	/// bauxite
	api_item4: i64,
	/// devmat
	api_item5: i64,
	/// construction dock id
	api_kdock_id: i64,
	/// high-speed construction
	api_highspeed: i64,
	/// large ship construction
	api_large_flag: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resp {
	api_create_flag: i64,
	api_get_items: Vec<GetItem>,
	api_material: Vec<i64>,
	api_unset_items: Option<Vec<UnsetItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetItem {
	api_id: i64,
	api_slotitem_id: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnsetItem {
	api_slot_list: Vec<i64>,
	api_type3: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let codex = state.codex();

	let mut costs = vec![
		(MaterialCategory::Fuel, params.api_item1),
		(MaterialCategory::Ammo, params.api_item2),
		(MaterialCategory::Steel, params.api_item3),
		(MaterialCategory::Bauxite, params.api_item4),
		(MaterialCategory::DevMat, params.api_item5),
	];

	if params.api_highspeed != 0 {
		costs.push((MaterialCategory::Torch, (1 + 9 * params.api_large_flag)));
	};

	let pool: Vec<Kc3rdShip> = codex
		.ship_extra
		.iter()
		.filter_map(|(_, info)| {
			if !info.buildable || (params.api_large_flag == 0 && !info.buildable_lsc) {
				None
			} else {
				Some(info.clone())
			}
		})
		.collect();

	let mut r = SmallRng::from_os_rng();

	let ship = pool.choose(&mut r).ok_or(ApiError::Internal(format!(
		"Failed to choose a ship from the pool of {} ships",
		pool.len()
	)))?;

	state
		.create_ship(
			pid,
			params.api_kdock_id,
			ship.api_id,
			params.api_large_flag > 0,
			params.api_highspeed > 0,
			&costs,
		)
		.await?;

	Ok(KcApiResponse::empty())
}
