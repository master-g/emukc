use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Account holds the user's account information in `EmuKC`.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Account {
	/// User ID
	pub uid: i64,

	/// User name
	pub name: String,

	/// Hashed password
	pub secret: String,

	/// Account creation time
	pub create_time: DateTime<Utc>,

	/// Last login time
	pub last_login: DateTime<Utc>,

	/// Last update time
	pub last_update: DateTime<Utc>,
}

impl Account {
	/// Create a new account
	///
	/// # Arguments
	///
	/// * `uid` - User ID
	/// * `name` - User name
	/// * `secret` - Hashed password
	pub fn new(uid: i64, name: String, secret: String) -> Self {
		Self {
			uid,
			name,
			secret,
			create_time: Utc::now(),
			last_login: Utc::now(),
			last_update: Utc::now(),
		}
	}
}
