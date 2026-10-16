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
    /// Sent only when the placed furniture comes with a port BGM.
    api_bgm_id: Option<i64>,
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

    // 0: the piece that came with a BGM was replaced by one without; back to the default.
    if let Some(bgm_id) = params.api_bgm_id {
        let bgm_id = if bgm_id == 0 {
            101
        } else {
            bgm_id
        };
        state.update_port_bgm(pid, bgm_id).await?;
    }

    Ok(KcApiResponse::empty())
}
