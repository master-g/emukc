use axum::response::{IntoResponse, Response};
use emukc_internal::prelude::AccountError;
use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
	#[error("Missing token")]
	MissingToken,

	#[error("Invalid token")]
	InvalidToken,

	#[error("Not fount")]
	NotFound,

	#[error("Internal error: {0}")]
	Internal(String),

	#[error("Unknown error {0}")]
	Unknown(String),

	#[error("Validation error: {0}")]
	Validation(#[from] validator::ValidationErrors),
}

impl From<AccountError> for ApiError {
	fn from(value: AccountError) -> Self {
		match value {
			AccountError::TokenInvalid | AccountError::TokenExpired => Self::InvalidToken,
			AccountError::UserNotFound | AccountError::ProfileNotFound => Self::NotFound,
			AccountError::Db(db_err) => Self::Internal(db_err.to_string()),
			_ => Self::Unknown(value.to_string()),
		}
	}
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		match self {
			ApiError::MissingToken => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),
			ApiError::InvalidToken => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
			ApiError::NotFound => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
			ApiError::Internal(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
			ApiError::Unknown(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
			ApiError::Validation(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
		}
	}
}
