use axum::Extension;

use crate::net::{
	AppState,
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
};
use emukc_internal::prelude::*;

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
) -> KcApiResult {
	let pid = session.profile.id;
	let fleets = state.get_fleets(pid).await?;

	let mut deck_params = Vec::new();

	for _ in fleets {
		let param = KcApiDeckParam {
			api_seiku_value: 0,
			api_tp_value: 0,
			api_atp_value: None,
		};
		deck_params.push(param);
	}

	Ok(KcApiResponse::success(&KcApiChartAdditionalInfo {
		api_deck_param: deck_params,
	}))
}
