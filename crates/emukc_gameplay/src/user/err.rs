use thiserror::Error;

use crate::err::GameplayError;

#[derive(Debug, Error)]
pub enum UserError {
	#[error("The username is already taken.")]
	UsernameTaken,

	#[error("Username too short.")]
	UsernameTooShort,

	#[error("Password too short.")]
	PasswordTooShort,

	#[error("Invalid username or password.")]
	InvalidUsernameOrPassword,

	#[error("Token invalid.")]
	TokenInvalid,

	#[error("Token expired.")]
	TokenExpired,

	#[error("User not found.")]
	UserNotFound,

	#[error("Profile not found.")]
	ProfileNotFound,

	#[error("Profile already exists.")]
	ProfileExists,

	#[error("Database error: {0}")]
	Db(#[from] emukc_db::sea_orm::DbErr),

	#[error("Gameplay error: {0}")]
	Gameplay(#[from] GameplayError),
}
