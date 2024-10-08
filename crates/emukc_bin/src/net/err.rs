use axum::response::{IntoResponse, Response};
use emukc_internal::prelude::{GameplayError, UserError};
use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
	#[error("Missing token")]
	MissingToken,

	#[error("Invalid token")]
	InvalidToken,

	#[error("Not found")]
	NotFound,

	#[error("Internal error: {0}")]
	Internal(String),

	#[error("Unknown error {0}")]
	Unknown(String),

	#[error("Validation error: {0}")]
	Validation(#[from] validator::ValidationErrors),
}

impl From<UserError> for ApiError {
	fn from(value: UserError) -> Self {
		match value {
			UserError::TokenInvalid | UserError::TokenExpired => Self::InvalidToken,
			UserError::UserNotFound | UserError::ProfileNotFound => Self::NotFound,
			UserError::Db(db_err) => Self::Internal(db_err.to_string()),
			_ => Self::Unknown(value.to_string()),
		}
	}
}

impl From<GameplayError> for ApiError {
	fn from(value: GameplayError) -> Self {
		match value {
			GameplayError::ProfileNotFound(_) => Self::NotFound,
			GameplayError::Db(db_err) => Self::Internal(db_err.to_string()),
			GameplayError::InvalidMaterialCategory(e) => Self::Internal(e.to_string()),
			GameplayError::ManifestNotFound(e) => Self::Internal(e.to_string()),
			GameplayError::CapacityExceeded(e) => Self::Internal(e.to_string()),
			GameplayError::ShipCreationFailed(e) => Self::Internal(e.to_string()),
			GameplayError::Codex(e) => Self::Internal(e.to_string()),
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
