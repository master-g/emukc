use chrono::{DateTime, Utc};
use emukc_crypto::SimpleHash;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Account {
	pub uid: i64,
	pub name: String,
	pub secret: String,
	pub create_time: DateTime<Utc>,
	pub last_login: DateTime<Utc>,
	pub last_update: DateTime<Utc>,
	pub access_token: Option<Token>,
	pub refresh_token: Option<Token>,
}

impl Account {
	pub fn new(uid: i64, name: String, secret: String) -> Self {
		Self {
			uid,
			name,
			secret,
			create_time: Utc::now(),
			last_login: Utc::now(),
			last_update: Utc::now(),
			access_token: None,
			refresh_token: None,
		}
	}
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Token {
	pub token: String,
	pub expire: DateTime<Utc>,
}

impl Token {
	pub fn issue(salt: &str, expire_duration: std::time::Duration) -> Self {
		let token = Uuid::new_v4().as_bytes().simple_hash_salted(salt);
		let expire = Utc::now() + expire_duration;

		Self {
			token,
			expire,
		}
	}

	pub fn is_valid(&self) -> bool {
		self.expire > Utc::now()
	}

	pub fn is_expired(&self) -> bool {
		!self.is_valid()
	}
}
