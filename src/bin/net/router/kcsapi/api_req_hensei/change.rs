use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	err::ApiError,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
	/// 0: disban all except flagship
	/// x: operate on ship x
	api_ship_idx: i64,
	/// fleet id
	api_id: i64,
	/// -1: for disband
	/// -2: disband all except flagship
	/// x: target ship id
	api_ship_id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
struct Resp {
	#[serde(skip_serializing_if = "Option::is_none")]
	api_change_count: Option<i64>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;
	let fleet = state.get_fleet(pid, params.api_id).await?;

	let origin_ship_idx = if params.api_ship_id < 0 {
		params.api_ship_id
	} else {
		let pos = fleet.ships.iter().position(|s| *s == params.api_ship_id);
		pos.map(|p| p as i64).unwrap_or(-1)
	};

	let source_ship_id = fleet.ships.get(params.api_ship_idx as usize).ok_or_else(|| {
		ApiError::NotFound(format!(
			"ship idx {} not found in fleet {}",
			params.api_ship_idx, params.api_id
		))
	})?;

	let (mut new_fleet, count) = if origin_ship_idx == -2 {
		// disband all except flagship
		let count = fleet.ships.iter().skip(1).fold(0, |acc, &s| {
			if s > 0 {
				acc + 1
			} else {
				acc
			}
		});
		let count = if count > 0 {
			Some(count)
		} else {
			None
		};
		([fleet.ships[0], -1, -1, -1, -1, -1], count)
	} else {
		let mut nf = fleet.ships.clone();
		nf[params.api_ship_idx as usize] = params.api_ship_id;
		if origin_ship_idx > 0 {
			// swap
			nf[origin_ship_idx as usize] = *source_ship_id;
		}
		(nf, None)
	};

	new_fleet.move_value_to_end(-1);

	state.update_fleet_ships(pid, params.api_id, &new_fleet).await?;

	Ok(KcApiResponse::success(&Resp {
		api_change_count: count,
	}))
}
