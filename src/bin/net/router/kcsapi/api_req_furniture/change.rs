use axum::{Extension, Form};
use emukc::model::profile::furniture::FurnitureConfig;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

#[derive(Deserialize, Serialize, Debug)]
pub(super) struct Params {
	api_floor: i64,
	api_wallpaper: i64,
	api_window: i64,
	api_wallhanging: i64,
	api_shelf: i64,
	api_desk: i64,
	api_season: Option<i64>,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	state
		.update_furniture_config(
			pid,
			&FurnitureConfig {
				floor: params.api_floor,
				wallpaper: params.api_wallpaper,
				window: params.api_window,
				wall_hanging: params.api_wallhanging,
				shelf: params.api_shelf,
				desk: params.api_desk,
				season: params.api_season.unwrap_or(0),
			},
		)
		.await?;

	Ok(KcApiResponse::empty())
}
