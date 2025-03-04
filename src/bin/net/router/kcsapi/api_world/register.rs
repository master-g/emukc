use axum::Form;
use serde::{Deserialize, Serialize};

use crate::net::{
	AppState,
	resp::{KcApiError, KcApiResponse, KcApiResult},
};

use emukc_internal::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub(super) struct RegisterForm {
	api_verno: i64,
	api_dmmuser_id: i64,
	api_world_id: i64,
}

pub(super) async fn handler(state: AppState, Form(params): Form<RegisterForm>) -> KcApiResult {
	match state.select_world(params.api_dmmuser_id, params.api_world_id).await {
		Ok(_) => {
			trace!("user {} selected world {}", params.api_dmmuser_id, params.api_world_id);
			Ok(KcApiResponse::empty())
		}
		Err(e) => {
			error!("failed to select world for user {}: {:?}", params.api_dmmuser_id, e);
			Err(KcApiError(e.into()))
		}
	}
}
