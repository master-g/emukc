use chrono::{DateTime, Utc};
use emukc_crypto::SimpleHash;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum TokenType {
	Access,
	Refresh,
}

/// Token is a struct that holds the token information.
/// It is used to issue and verify the token.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Token {
	/// uid
	pub uid: i64,

	/// Token type
	pub typ: TokenType,

	/// Token string
	pub token: String,

	/// Token expiration time
	pub expire: DateTime<Utc>,
}

impl Token {
	/// Issue a new token
	///
	/// # Arguments
	///
	/// * `typ` - Token type
	/// * `uid` - User ID
	/// * `salt` - Salt for hashing
	/// * `expire_duration` - Token expiration duration
	pub fn issue(
		typ: TokenType,
		uid: i64,
		salt: &str,
		expire_duration: std::time::Duration,
	) -> Self {
		let token = Uuid::new_v4().as_bytes().simple_hash_salted(salt);
		let expire = Utc::now() + expire_duration;

		Self {
			typ,
			uid,
			token,
			expire,
		}
	}

	/// Check if the token is valid
	pub fn is_valid(&self) -> bool {
		self.expire > Utc::now()
	}

	/// Check if the token is expired
	pub fn is_expired(&self) -> bool {
		!self.is_valid()
	}
}
