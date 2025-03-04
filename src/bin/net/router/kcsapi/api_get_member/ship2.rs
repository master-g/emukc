use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use emukc_internal::prelude::*;

use crate::net::{
	AppState,
	auth::GameSession,
	err::ApiError,
	resp::{KcApiResponse, KcApiResult},
};

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct Params {
	/// seems to be a constant value, 2 it is
	spi_sort_order: i64,

	api_shipid: Option<i64>,

	/// seems to be a constant value, 5 it is
	api_sort_key: i64,
}

#[derive(Serialize)]
struct Resp {
	api_data_deck: Vec<KcApiDeckPort>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let ships = if let Some(ship_id) = params.api_shipid {
		let ship = state
			.find_ship(ship_id)
			.await?
			.ok_or(ApiError::NotFound(format!("ship with id {} not found", ship_id)))?;
		vec![ship]
	} else {
		state.get_ships(pid).await?
	};

	let fleets = state.get_fleets(pid).await?;
	let api_data_deck = fleets.into_iter().map(std::convert::Into::into).collect();

	Ok(KcApiResponse::success_extra(
		&ships,
		&Some(Resp {
			api_data_deck,
		}),
	))
}
