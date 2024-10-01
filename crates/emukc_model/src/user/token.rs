use chrono::{DateTime, Utc};
use emukc_crypto::SimpleHash;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const ACCESS_TOKEN_SALT: &str = "emukc_access_token";
const REFRESH_TOKEN_SALT: &str = "emukc_refresh_token";
const SESSION_TOKEN_SALT: &str = "emukc_session_token";

/// Token type
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TokenType {
	/// Access token
	Access,
	/// Refresh token
	Refresh,
	/// Game Session
	Session,
}

impl TokenType {
	/// Get the duration of the token
	pub fn duration(&self) -> std::time::Duration {
		match self {
			TokenType::Access => chrono::Duration::days(7).to_std().unwrap(),
			TokenType::Refresh => chrono::Duration::days(30).to_std().unwrap(),
			TokenType::Session => chrono::Duration::hours(24).to_std().unwrap(),
		}
	}
}

/// Token is a struct that holds the token information.
/// It is used to issue and verify the token.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Token {
	/// uid
	pub uid: i64,

	/// profile id
	pub profile_id: i64,

	/// Token type
	pub typ: TokenType,

	/// Token string
	pub token: String,

	/// Token expiration time
	pub expire: DateTime<Utc>,
}

impl Token {
	/// Issue an access token
	///
	/// access token will have a 7 days expiration time
	///
	/// # Arguments
	///
	/// * `uid` - User ID
	pub fn issue_access(uid: i64) -> Self {
		Self::issue(uid, 0, TokenType::Access)
	}

	/// Issue a refresh token
	///
	/// refresh token will have a 30 days expiration time
	///
	/// # Arguments
	///
	/// * `uid` - User ID
	pub fn issue_refresh(uid: i64) -> Self {
		Self::issue(uid, 0, TokenType::Refresh)
	}

	/// Issue a game session token
	///
	/// game session token will have a 1 hour expiration time
	///
	/// # Arguments
	///
	/// * `uid` - User ID
	/// * `profile_id` - Profile ID
	pub fn issue_session(uid: i64, profile_id: i64) -> Self {
		Self::issue(uid, profile_id, TokenType::Session)
	}

	/// Check if the token is expired
	pub fn is_expired(&self) -> bool {
		self.expire < Utc::now()
	}

	fn new_partial(salt: &str, span: std::time::Duration) -> (String, DateTime<Utc>) {
		let token = Uuid::new_v4().as_bytes().simple_hash_salted(salt);
		let expire = Utc::now() + span;

		(token, expire)
	}

	fn issue(uid: i64, profile_id: i64, typ: TokenType) -> Self {
		let (salt, span) = match typ {
			TokenType::Access => (ACCESS_TOKEN_SALT, typ.duration()),
			TokenType::Refresh => (REFRESH_TOKEN_SALT, typ.duration()),
			TokenType::Session => (SESSION_TOKEN_SALT, typ.duration()),
		};

		let (token, expire) = Self::new_partial(salt, span);

		Self {
			uid,
			profile_id,
			typ,
			token,
			expire,
		}
	}
}
