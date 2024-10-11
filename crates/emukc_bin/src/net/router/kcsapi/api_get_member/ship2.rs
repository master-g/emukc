use axum::Extension;
use serde::Serialize;

use emukc_internal::prelude::*;

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};

#[derive(Serialize)]
struct DeckPorts {
	api_data_deck: Vec<KcApiDeckPort>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;
	let ships = state.get_ships(pid).await?;
	let fleets = state.get_fleets(pid).await?;
	let api_data_deck = fleets.into_iter().map(|f| f.into()).collect();

	Ok(KcApiResponse::success_extra(
		&ships,
		&Some(DeckPorts {
			api_data_deck,
		}),
	))
}
