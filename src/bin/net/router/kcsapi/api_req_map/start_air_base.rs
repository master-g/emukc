use axum::Form;
use serde::Deserialize;

use crate::net::prelude::*;

/// Cells for each air corps of the area, comma separated; a corps that stays
/// home sends nothing.
#[derive(Deserialize, Default)]
pub(super) struct Params {
    #[serde(default)]
    pub(super) api_strike_point_1: String,
    #[serde(default)]
    pub(super) api_strike_point_2: String,
    #[serde(default)]
    pub(super) api_strike_point_3: String,
}

fn cells(list: &str) -> Result<Vec<i64>, GameplayError> {
    list.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse::<i64>().map_err(|_| {
                GameplayError::WrongType(format!("strike point `{part}` is not a cell number"))
            })
        })
        .collect()
}

pub(super) async fn handler(
    state: AppState,
    Pid(pid): Pid,
    Form(params): Form<Params>,
) -> KcApiResult {
    let strike_points = [
        cells(&params.api_strike_point_1)?,
        cells(&params.api_strike_point_2)?,
        cells(&params.api_strike_point_3)?,
    ];

    state.start_air_base(pid, &strike_points).await?;

    Ok(KcApiResponse::empty())
}
