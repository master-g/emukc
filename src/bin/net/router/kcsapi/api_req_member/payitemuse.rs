use axum::{Extension, Form};
use serde::{Deserialize, Serialize};

use crate::net::{
	auth::GameSession,
	resp::{KcApiResponse, KcApiResult},
	AppState,
};
use emukc_internal::prelude::*;

#[derive(Deserialize)]
pub(super) struct Params {
	api_payitem_id: i64,

	// 0: response.api_caution_flag will be 1 if the material will be capped by limit.
	// 1: response.api_caution_flag will be 0 if the material will be capped by limit.
	api_force_flag: i64,
}

#[derive(Serialize, Default)]
struct Resp {
	// 0: will not show caution dialog, 1: will show caution dialog
	api_caution_flag: i64,
}

pub(super) async fn handler(
	state: AppState,
	Extension(session): Extension<GameSession>,
	Form(params): Form<Params>,
) -> KcApiResult {
	let pid = session.profile.id;

	let caution =
		state.consume_pay_item(pid, params.api_payitem_id, params.api_force_flag == 1).await?;

	Ok(KcApiResponse::success(&Resp {
		api_caution_flag: if caution {
			1
		} else {
			0
		},
	}))
}
