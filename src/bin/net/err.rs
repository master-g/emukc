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

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

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
            UserError::UsernameTaken | UserError::ProfileExists => {
                Self::Conflict(value.to_string())
            }
            UserError::UsernameTooShort | UserError::PasswordTooShort => {
                Self::BadRequest(value.to_string())
            }
            UserError::InvalidUsernameOrPassword => Self::Unauthorized(value.to_string()),
            UserError::Db(db_err) => Self::Internal(db_err.to_string()),
            UserError::Gameplay(e) => Self::from(e),
        }
    }
}

impl From<GameplayError> for ApiError {
    fn from(value: GameplayError) -> Self {
        match value {
            GameplayError::ProfileNotFound(e) => Self::NotFound(e.to_string()),
            GameplayError::Db(db_err) => Self::Internal(db_err.to_string()),
            GameplayError::WrongType(e)
            | GameplayError::Insufficient(e)
            | GameplayError::QuestStatusInvalid(e) => Self::BadRequest(e),
            GameplayError::CapacityExceeded(e) => Self::BadRequest(e.to_string()),
            GameplayError::BadManifest(e) => Self::Internal(e),
            GameplayError::ManifestNotFound(e) | GameplayError::ShipCreationFailed(e) => {
                Self::Internal(e.to_string())
            }
            GameplayError::Codex(e) => Self::Internal(e.to_string()),
            GameplayError::EntryNotFound(e) | GameplayError::Locked(e) => Self::NotFound(e),
            GameplayError::Json(e) => Self::Internal(e.to_string()),
            GameplayError::Reward(e) => Self::Internal(e.to_string()),
        }
    }
}

impl From<CodexError> for ApiError {
    fn from(value: CodexError) -> Self {
        match value {
            CodexError::Io(e) => Self::Internal(e.to_string()),
            CodexError::Parse(e) => Self::Internal(e.to_string()),
            CodexError::Serde(e) => Self::Internal(e.to_string()),
            CodexError::NotFound(e) => Self::NotFound(e),
        }
    }
}

/// Text sent to clients in place of a server error's details, which go to the log.
pub const INTERNAL_ERROR_TEXT: &str = "Internal server error";

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            ApiError::MissingToken | ApiError::BadRequest(_) | ApiError::Validation(_) => {
                StatusCode::BAD_REQUEST
            }
            ApiError::InvalidToken | ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::Internal(_) | ApiError::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            error!("{self}");
            return (status, INTERNAL_ERROR_TEXT).into_response();
        }
        let body = match self {
            ApiError::NotFound(e)
            | ApiError::BadRequest(e)
            | ApiError::Unauthorized(e)
            | ApiError::Conflict(e) => e,
            _ => self.to_string(),
        };
        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use emukc_internal::db::sea_orm::DbErr;

    use super::*;

    fn status_of(err: impl Into<ApiError>) -> StatusCode {
        err.into().into_response().status()
    }

    #[test]
    fn user_errors_map_to_their_status() {
        let cases = [
            (UserError::UsernameTaken, StatusCode::CONFLICT),
            (UserError::ProfileExists, StatusCode::CONFLICT),
            (UserError::UsernameTooShort, StatusCode::BAD_REQUEST),
            (UserError::PasswordTooShort, StatusCode::BAD_REQUEST),
            (UserError::InvalidUsernameOrPassword, StatusCode::UNAUTHORIZED),
            (UserError::TokenInvalid, StatusCode::UNAUTHORIZED),
            (UserError::TokenExpired, StatusCode::UNAUTHORIZED),
            (UserError::UserNotFound, StatusCode::NOT_FOUND),
            (UserError::ProfileNotFound, StatusCode::NOT_FOUND),
            (UserError::Db(DbErr::Custom("db".into())), StatusCode::INTERNAL_SERVER_ERROR),
            (UserError::Gameplay(GameplayError::WrongType("w".into())), StatusCode::BAD_REQUEST),
            (UserError::Gameplay(GameplayError::EntryNotFound("e".into())), StatusCode::NOT_FOUND),
        ];
        for (err, expected) in cases {
            let name = format!("{err:?}");
            assert_eq!(status_of(err), expected, "{name}");
        }
    }

    #[test]
    fn gameplay_errors_map_to_their_status() {
        let cases = [
            (GameplayError::WrongType("w".into()), StatusCode::BAD_REQUEST),
            (GameplayError::Insufficient("i".into()), StatusCode::BAD_REQUEST),
            (GameplayError::CapacityExceeded(100), StatusCode::BAD_REQUEST),
            (GameplayError::QuestStatusInvalid("q".into()), StatusCode::BAD_REQUEST),
            (GameplayError::ProfileNotFound(1), StatusCode::NOT_FOUND),
            (GameplayError::EntryNotFound("e".into()), StatusCode::NOT_FOUND),
            (GameplayError::Locked("l".into()), StatusCode::NOT_FOUND),
            (GameplayError::Db(DbErr::Custom("db".into())), StatusCode::INTERNAL_SERVER_ERROR),
            (GameplayError::BadManifest("b".into()), StatusCode::INTERNAL_SERVER_ERROR),
            (GameplayError::ManifestNotFound(1), StatusCode::INTERNAL_SERVER_ERROR),
            (GameplayError::ShipCreationFailed(1), StatusCode::INTERNAL_SERVER_ERROR),
            (
                GameplayError::Codex(CodexError::NotFound("c".into())),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];
        for (err, expected) in cases {
            let name = format!("{err:?}");
            assert_eq!(status_of(err), expected, "{name}");
        }
    }

    #[tokio::test]
    async fn server_error_body_hides_the_database_message() {
        let resp =
            ApiError::from(UserError::Db(DbErr::Custom("secret-table".into()))).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(!body.contains("secret-table"), "body leaked the database message: {body}");
    }

    #[tokio::test]
    async fn client_error_body_keeps_the_message() {
        let resp = ApiError::from(UserError::UsernameTaken).into_response();
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("already taken"), "{body}");
    }
}
