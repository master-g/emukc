use axum::Form;
use emukc::model::profile::furniture::FurnitureConfig;
use serde::{Deserialize, Serialize};

use crate::net::prelude::*;

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
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
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
