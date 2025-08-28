use axum::response::{IntoResponse, Response};
use emukc_internal::{
	model::codex::CodexError,
	prelude::{GameplayError, UserError},
};
use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
	#[error("Missing token")]
	MissingToken,

	#[error("Invalid token")]
	InvalidToken,

	#[error("Not found: {0}")]
	NotFound(String),

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
			UserError::UserNotFound => Self::NotFound("User not found".to_string()),
			UserError::ProfileNotFound => Self::NotFound("Profile not found".to_string()),
			UserError::Db(db_err) => Self::Internal(db_err.to_string()),
			_ => Self::Unknown(value.to_string()),
		}
	}
}

impl From<GameplayError> for ApiError {
	fn from(value: GameplayError) -> Self {
		match value {
			GameplayError::ProfileNotFound(e) => Self::NotFound(e.to_string()),
			GameplayError::Db(db_err) => Self::Internal(db_err.to_string()),
			GameplayError::BadManifest(e)
			| GameplayError::WrongType(e)
			| GameplayError::Insufficient(e)
			| GameplayError::QuestStatusInvalid(e) => Self::Internal(e),
			GameplayError::ManifestNotFound(e) | GameplayError::CapacityExceeded(e) => {
				Self::Internal(e.to_string())
			}
			GameplayError::ShipCreationFailed(e) => Self::Internal(e.to_string()),
			GameplayError::Codex(e) => Self::Internal(e.to_string()),
			GameplayError::EntryNotFound(e) => Self::NotFound(e),
			GameplayError::Json(e) => Self::Internal(e.to_string()),
		}
	}
}

impl From<CodexError> for ApiError {
	fn from(value: CodexError) -> Self {
		match value {
			CodexError::AlreadyExist(e) => Self::Internal(e),
			CodexError::Io(e) => Self::Internal(e.to_string()),
			CodexError::Parse(e) => Self::Internal(e.to_string()),
			CodexError::Serde(e) => Self::Internal(e.to_string()),
			CodexError::NotFound(e) => Self::NotFound(e),
		}
	}
}

impl IntoResponse for ApiError {
	fn into_response(self) -> Response {
		match self {
			ApiError::MissingToken => (StatusCode::BAD_REQUEST, self.to_string()).into_response(),
			ApiError::InvalidToken => (StatusCode::UNAUTHORIZED, self.to_string()).into_response(),
			ApiError::NotFound(e) => (StatusCode::NOT_FOUND, e).into_response(),
			ApiError::Internal(e) | ApiError::Unknown(e) => {
				(StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
			}
			ApiError::Validation(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
		}
	}
}
