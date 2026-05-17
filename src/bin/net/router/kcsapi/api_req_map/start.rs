use axum::{Extension, Form};
use serde::Deserialize;

use crate::net::{
    AppState,
    auth::GameSession,
    resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

use super::projection::project_start;

#[derive(Deserialize)]
pub(super) struct Params {
    pub(super) api_deck_id: i64,
    pub(super) api_maparea_id: i64,
    pub(super) api_mapinfo_no: i64,
    #[serde(default)]
    #[expect(dead_code)]
    pub(super) api_serial_cid: String,
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;
    let resp = state
        .start_sortie(pid, params.api_deck_id, params.api_maparea_id, params.api_mapinfo_no)
        .await?;

    Ok(KcApiResponse::success(&project_start(resp)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_deserialize_without_formation_id() {
        let params: Params = serde_urlencoded::from_str(
				"api_token=test&api_verno=1&api_maparea_id=1&api_mapinfo_no=1&api_deck_id=1&api_serial_cid=123",
			)
			.unwrap();

        assert_eq!(params.api_maparea_id, 1);
        assert_eq!(params.api_mapinfo_no, 1);
        assert_eq!(params.api_deck_id, 1);
    }

    #[test]
    fn params_ignore_unknown_formation_id_field() {
        let params: Params = serde_urlencoded::from_str(
            "api_maparea_id=1&api_mapinfo_no=1&api_deck_id=1&api_formation_id=3",
        )
        .unwrap();

        assert_eq!(params.api_maparea_id, 1);
    }
}
