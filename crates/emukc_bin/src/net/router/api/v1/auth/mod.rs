use axum::{routing::post, Json, Router};
use emukc_internal::model::user::token::Token;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::net::AppState;

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
) -> Result<Json<AuthResponse>, Error> {
	params.validate().map_err(Error::from)?;

	let user = User::sign_in(&db, &params.username, &params.password).await?;

	Ok(Json(AuthResponse {
		uid: user.uid(),
		access_token: user.access_token().unwrap(),
		refresh_token: user.refresh_token().unwrap(),
	}))
}

async fn sign_up(
	db: UserDB,
	Json(params): Json<SignParameter>,
) -> Result<Json<AuthResponse>, Error> {
	params.validate().map_err(Error::from)?;

	let user = User::sign_up(&db, &params.username, &params.password).await?;

	Ok(Json(AuthResponse {
		uid: user.uid(),
		access_token: user.access_token().unwrap(),
		refresh_token: user.refresh_token().unwrap(),
	}))
}
