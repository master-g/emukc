//! Deal with account related stuff.

use emukc_crypto::PasswordCrypto;
use emukc_db::{
	entity::user::{account, token},
	sea_orm::{entity::*, query::*},
};
use emukc_model::user::{account::Account, token::Token};
use emukc_time::chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::prelude::Gameplay;

const MIN_USERNAME_LEN: usize = 4;
const MIN_PASSWORD_LEN: usize = 7;

#[derive(Debug, Error)]
pub enum AccountError {
	#[error("The username is already taken.")]
	UsernameTaken,

	#[error("Username too short.")]
	UsernameTooShort,

	#[error("Password too short.")]
	PasswordTooShort,

	#[error("Database error: {0}")]
	Db(#[from] emukc_db::sea_orm::DbErr),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignUpResult {
	pub account: Account,
	pub access_token: Token,
	pub refresh_token: Token,
}

impl Gameplay {
	/// Create a new account.
	///
	/// # Arguments
	///
	/// * `username` - The username of the new account.
	/// * `password` - The password of the new account.
	pub async fn sign_up(
		&self,
		username: &str,
		password: &str,
	) -> Result<SignUpResult, AccountError> {
		let db = &*self.db;

		let tx = db.begin().await?;

		let model =
			account::Entity::find().filter(account::Column::Name.eq(username)).one(&tx).await?;

		if model.is_some() {
			return Err(AccountError::UsernameTaken);
		}

		if username.len() < MIN_USERNAME_LEN {
			return Err(AccountError::UsernameTooShort);
		}

		if password.len() < MIN_PASSWORD_LEN {
			return Err(AccountError::PasswordTooShort);
		}

		let secret = password.hash_password();

		let now = Utc::now();
		let model = account::ActiveModel {
			uid: ActiveValue::NotSet,
			name: ActiveValue::Set(username.to_string()),
			secret: ActiveValue::Set(secret),
			create_time: ActiveValue::Set(now),
			last_login: ActiveValue::Set(now),
			last_update: ActiveValue::Set(now),
		};
		let model = model.insert(&tx).await?;

		// issue new tokens
		let access_token = Token::issue_access(model.uid);
		let refresh_token = Token::issue_refresh(model.uid);

		// insert tokens
		{
			token::ActiveModel {
				uid: ActiveValue::Set(access_token.uid),
				profile_id: ActiveValue::NotSet,
				typ: ActiveValue::Set(token::TokenTypeDef::Access),
				token: ActiveValue::Set(access_token.token),
				expire: ActiveValue::Set(access_token.expire),
			}
			.save(&tx)
			.await?;
		}

		// final commit
		tx.commit().await?;

		todo!()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_new_account() {
		let gameplay = Gameplay::new_mock().await;
		let db = &*gameplay.db;

		let username = "admin";
		let password = "abcd123";

		let result = gameplay.sign_up(username, password).await.unwrap();

		println!("{:?}", result);

		// assert_eq!(model.name, username);
	}
}
