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
    #[serde(default = "default_formation_id")]
    pub(super) api_formation_id: i64,
    pub(super) api_maparea_id: i64,
    pub(super) api_mapinfo_no: i64,
    #[serde(default)]
    #[allow(dead_code)]
    pub(super) api_serial_cid: String,
}

fn default_formation_id() -> i64 {
    1
}

pub(super) async fn handler(
    state: AppState,
    Extension(session): Extension<GameSession>,
    Form(params): Form<Params>,
) -> KcApiResult {
    let pid = session.profile.id;
    let resp = state
        .start_sortie(
            pid,
            params.api_deck_id,
            params.api_maparea_id,
            params.api_mapinfo_no,
            params.api_formation_id,
        )
        .await?;

    Ok(KcApiResponse::success(&project_start(resp)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_allow_missing_formation_id_and_default_to_line_ahead() {
        let params: Params = serde_urlencoded::from_str(
			"api_token=test&api_verno=1&api_maparea_id=1&api_mapinfo_no=1&api_deck_id=1&api_serial_cid=123",
		)
		.unwrap();

        assert_eq!(params.api_formation_id, 1);
        assert_eq!(params.api_maparea_id, 1);
        assert_eq!(params.api_mapinfo_no, 1);
        assert_eq!(params.api_deck_id, 1);
    }

    #[test]
    fn params_preserve_explicit_formation_id_when_present() {
        let params: Params = serde_urlencoded::from_str(
            "api_maparea_id=1&api_mapinfo_no=1&api_deck_id=1&api_formation_id=3",
        )
        .unwrap();

        assert_eq!(params.api_formation_id, 3);
    }
}
