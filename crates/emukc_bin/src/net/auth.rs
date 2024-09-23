use axum::{
	async_trait,
	body::Body,
	extract::{FromRef, FromRequest, FromRequestParts, Request},
	middleware::Next,
	response::{IntoResponse, Response},
	Form, RequestPartsExt,
};
use emukc_internal::model::profile::Profile;
use http::{header, request::Parts, StatusCode};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};

use super::AppState;

#[derive(Clone)]
pub(super) struct AuthUserProfile(pub Profile);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUserProfile
where
	State: FromRef<S>,
	S: Send + Sync,
{
	type Rejection = Box<dyn std::error::Error>;

	async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
		let state = State::from_ref(state);
		let token_names = ["access_token", "token", "st", "api_token"];

		// extract token from query first
		let token = parts
			.uri
			.query()
			.and_then(|query| {
				url::form_urlencoded::parse(query.as_bytes())
					.find(|(key, _)| token_names.contains(&key.as_ref()))
					.map(|(_, value)| value.to_string())
			})
			.or_else(|| {
				parts
					.headers
					.get(header::AUTHORIZATION)
					.and_then(|value| value.to_str().ok())
					.and_then(|value| {
						if value.starts_with("bearer ") || value.starts_with("Bearer ") {
							Some(value[7..].to_string())
						} else {
							None
						}
					})
			})
			.ok_or_else(|| Error::Auth("missing".to_string()))?;

		todo!()
	}
}
