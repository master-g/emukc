use axum::{Json, Router, routing::post};
use emukc_internal::prelude::*;
use serde::{Deserialize, Serialize};

use crate::net::{AppState, err::ApiError};

pub(super) fn router() -> Router {
	axum::Router::new().route("/add", post(add))
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct AddParams {
	profile_id: i64,
	ship_id: Vec<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct AddResp {
	ships: Vec<KcApiShip>,
}

pub(super) async fn add(
	state: AppState,
	Json(params): Json<AddParams>,
) -> Result<Json<AddResp>, ApiError> {
	let mut ships = vec![];

	for ship_id in params.ship_id {
		let ship = state.add_ship(params.profile_id, ship_id).await?;
		ships.push(ship);
	}

	Ok(Json(AddResp {
		ships,
	}))
}
