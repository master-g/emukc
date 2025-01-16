use axum::{
	body::Body,
	extract::{FromRef, FromRequest, FromRequestParts, Request},
	middleware::Next,
	response::{IntoResponse, Response},
	Form, RequestPartsExt,
};
use emukc_internal::{
	model::{profile::Profile, user::account::Account},
	prelude::{AccountOps, AuthInfo},
};
use http::{header, request::Parts, StatusCode};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};

use crate::state::State;

use super::{err::ApiError, AppState};

#[allow(unused)]
#[derive(Clone)]
pub(super) struct AuthAccount(pub Account);

impl<S> FromRequestParts<S> for AuthAccount
where
	State: FromRef<S>,
	S: Send + Sync,
{
	type Rejection = ApiError;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let state = State::from_ref(state);

		// extract token from query first
		let raw_token = find_token_in_parts(parts).ok_or(ApiError::MissingToken)?;

		match state.auth(&raw_token).await {
			Ok(AuthInfo::Account(account)) => Ok(Self(account)),
			Ok(AuthInfo::Profile(_)) => {
				info!("expected access token, got game session token");
				Err(ApiError::InvalidToken)
			}
			Err(e) => return Err(e.into()),
		}
	}
}

#[allow(unused)]
pub(super) async fn auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
	let (mut parts, body) = request.into_parts();

	let state = parts.extract::<AppState>().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
	let state = State::from_ref(&state);
	let auth_user = parts
		.extract_with_state::<AuthAccount, State>(&state)
		.await
		.map_err(|_| StatusCode::UNAUTHORIZED)?;

	parts.extensions.insert(auth_user);

	Ok(next.run(Request::from_parts(parts, body)).await)
}

#[derive(Debug, Clone)]
pub(super) struct GameSession {
	pub token: String,
	pub profile: Profile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct KcsApiFormWithToken {
	api_token: String,
}

impl<S> FromRequest<S> for KcsApiFormWithToken
where
	S: Send + Sync,
{
	type Rejection = (StatusCode, String);

	async fn from_request(request: Request, _state: &S) -> Result<Self, Self::Rejection> {
		let Form(form) = Form::<Self>::from_request(request, _state)
			.await
			.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

		Ok(form)
	}
}

impl<S> FromRequest<S> for GameSession
where
	State: FromRef<S>,
	S: Send + Sync + 'static,
{
	type Rejection = ApiError;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
		let state = State::from_ref(state);

		let Ok(form) = KcsApiFormWithToken::from_request(req, &state).await else {
			return Err(ApiError::MissingToken);
		};
		let token = form.api_token;

		match state.auth(&token).await {
			Ok(AuthInfo::Profile(profile)) => Ok(Self {
				token,
				profile,
			}),
			Ok(AuthInfo::Account(_)) => {
				info!("expected game session token, got access token");
				Err(ApiError::InvalidToken)
			}
			Err(e) => return Err(e.into()),
		}
	}
}

async fn extract_kcs_api_game_session(
	request: Request,
) -> Result<(GameSession, Request), Response> {
	let (mut parts, body) = request.into_parts();
	let state = parts
		.extract::<AppState>()
		.await
		.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;
	let state = State::from_ref(&state);

	let (token, req) = if let Some(token) = find_token_in_parts(&parts) {
		(token, Request::from_parts(parts, body))
	} else {
		let bytes = body
			.collect()
			.await
			.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response())?
			.to_bytes();

		let form = serde_urlencoded::from_bytes::<KcsApiFormWithToken>(&bytes)
			.map_err(|err| (StatusCode::BAD_REQUEST, err.to_string()).into_response())?;
		(form.api_token, Request::from_parts(parts, Body::from(bytes)))
	};

	match state.auth(&token).await {
		Ok(AuthInfo::Profile(profile)) => Ok((
			GameSession {
				token,
				profile,
			},
			req,
		)),
		Ok(AuthInfo::Account(_)) => {
			info!("expected game session token, got access token");
			Err((
				StatusCode::UNAUTHORIZED,
				"expected game session token, got access token".to_string(),
			)
				.into_response())
		}
		Err(e) => Err(ApiError::from(e).into_response()),
	}
}

#[allow(unused)]
pub(super) async fn kcs_api_auth_middleware(
	request: Request,
	next: Next,
) -> Result<Response, StatusCode> {
	let (auth_user, request) =
		extract_kcs_api_game_session(request).await.map_err(|e| e.status())?;
	let (mut parts, body) = request.into_parts();
	parts.extensions.insert(auth_user);

	Ok(next.run(Request::from_parts(parts, body)).await)
}

fn find_token_in_parts(parts: &Parts) -> Option<String> {
	let token_names = ["token", "st", "api_token"];
	parts
		.uri
		.query()
		.and_then(|query| {
			url::form_urlencoded::parse(query.as_bytes())
				.find(|(key, _)| token_names.contains(&key.as_ref()))
				.map(|(_, value)| value.to_string())
		})
		.or_else(|| {
			parts.headers.get(header::AUTHORIZATION).and_then(|value| value.to_str().ok()).and_then(
				|value| {
					if value.starts_with("bearer ") || value.starts_with("Bearer ") {
						Some(value[7..].to_string())
					} else {
						None
					}
				},
			)
		})
}
