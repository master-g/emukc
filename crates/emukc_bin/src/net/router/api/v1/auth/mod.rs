use axum::{routing::post, Json, Router};
use emukc_internal::{model::user::token::Token, prelude::AccountOps};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::net::{err::ApiError, AppState};

pub(super) fn router() -> Router {
	Router::new().route("/sign-in", post(sign_in)).route("/sign-up", post(sign_up))
}

#[derive(Serialize, Deserialize, Debug, Validate)]
struct SignParameter {
	#[validate(length(min = 5, max = 22))]
	username: String,
	#[validate(length(min = 7, max = 20))]
	password: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AuthResponse {
	uid: i64,
	access_token: Token,
	refresh_token: Token,
}

#[derive(Serialize, Deserialize, Debug, Validate)]
struct RenewRequest {
	#[validate(length(equal = 44))]
	access_token: String,
	#[validate(length(equal = 44))]
	refresh_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct RenewResponse {
	access_token: Token,
}

async fn sign_in(
	state: AppState,
	Json(params): Json<SignParameter>,
) -> Result<Json<AuthResponse>, ApiError> {
	params.validate().map_err(ApiError::from)?;

	let account = state.sign_in(&params.username, &params.password).await?;

	Ok(Json(AuthResponse {
		uid: account.account.uid,
		access_token: account.access_token,
		refresh_token: account.refresh_token,
	}))
}

async fn sign_up(
	state: AppState,
	Json(params): Json<SignParameter>,
) -> Result<Json<AuthResponse>, ApiError> {
	params.validate().map_err(ApiError::from)?;

	let account = state.sign_up(&params.username, &params.password).await?;

	Ok(Json(AuthResponse {
		uid: account.account.uid,
		access_token: account.access_token,
		refresh_token: account.refresh_token,
	}))
}
