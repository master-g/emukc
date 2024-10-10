//! Deal with account related stuff.

use emukc_crypto::PasswordCrypto;
use emukc_db::{
	entity::{
		profile,
		user::{account, token},
	},
	sea_orm::{entity::*, query::*},
};
use emukc_model::{
	profile::Profile,
	user::{
		account::Account,
		token::{Token, TokenType},
	},
};
use emukc_time::chrono::Utc;
use prelude::async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::gameplay::HasContext;

use super::{
	auth::{issue_token, verify_access_token},
	UserError,
};

const MIN_USERNAME_LEN: usize = 4;
const MIN_PASSWORD_LEN: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountInfo {
	pub account: Account,
	pub access_token: Token,
	pub refresh_token: Token,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthInfo {
	Account(Account),
	Profile(Profile),
}

/// A trait for account related gameplay.
#[async_trait]
pub trait AccountOps {
	/// Create a new account.
	///
	/// # Arguments
	///
	/// * `username` - The username of the new account.
	/// * `password` - The password of the new account.
	async fn sign_up(&self, username: &str, password: &str) -> Result<AccountInfo, UserError>;

	/// Sign in with username and password.
	///
	/// # Arguments
	///
	/// * `username` - The username of the account.
	/// * `password` - The password of the account.
	async fn sign_in(&self, username: &str, password: &str) -> Result<AccountInfo, UserError>;

	/// Authenticate with an access token.
	///
	/// # Arguments
	///
	/// * `token` - The token to authenticate with, usually an access token, or game session token.
	async fn auth(&self, token: &str) -> Result<AuthInfo, UserError>;

	/// Logout with an access token.
	///
	/// # Arguments
	///
	/// * `access_token` - The access token to logout with.
	async fn logout(&self, access_token: &str) -> Result<(), UserError>;

	/// Remove an account and all its data.
	///
	/// # Arguments
	///
	/// * `username` - The username of the account.
	/// * `password` - The password of the account.
	async fn delete_account(&self, username: &str, password: &str) -> Result<(), UserError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> AccountOps for T {
	async fn sign_up(&self, username: &str, password: &str) -> Result<AccountInfo, UserError> {
		let db = self.db();

		let tx = db.begin().await?;

		let model =
			account::Entity::find().filter(account::Column::Name.eq(username)).one(&tx).await?;

		if model.is_some() {
			return Err(UserError::UsernameTaken);
		}

		if username.len() < MIN_USERNAME_LEN {
			return Err(UserError::UsernameTooShort);
		}

		if password.len() < MIN_PASSWORD_LEN {
			return Err(UserError::PasswordTooShort);
		}

		let secret = password.hash_password();

		let now = Utc::now();
		let model = account::ActiveModel {
			uid: ActiveValue::NotSet,
			name: ActiveValue::Set(username.to_string()),
			secret: ActiveValue::Set(secret),
			create_time: ActiveValue::Set(now),
			last_login: ActiveValue::Set(now),
		};
		let model = model.insert(&tx).await?;

		// issue new tokens
		let access_token = issue_token(&tx, model.uid, 0, TokenType::Access).await?;
		let refresh_token = issue_token(&tx, model.uid, 0, TokenType::Refresh).await?;

		// final commit
		tx.commit().await?;

		Ok(AccountInfo {
			account: model.into(),
			access_token,
			refresh_token,
		})
	}

	/// Sign in with username and password.
	///
	/// # Arguments
	///
	/// * `username` - The username of the account.
	/// * `password` - The password of the account.
	async fn sign_in(&self, username: &str, password: &str) -> Result<AccountInfo, UserError> {
		let db = self.db();
		let tx = db.begin().await?;

		let model = account::Entity::find()
			.filter(account::Column::Name.eq(username))
			.one(&tx)
			.await?
			.ok_or_else(|| UserError::InvalidUsernameOrPassword)?;

		if !password.verify_password(&model.secret) {
			return Err(UserError::InvalidUsernameOrPassword);
		}

		let mut active_model: account::ActiveModel = model.into();
		active_model.last_login = ActiveValue::Set(Utc::now());

		let model = active_model.update(&tx).await?;

		let access_token = issue_token(&tx, model.uid, 0, TokenType::Access).await?;
		let refresh_token = issue_token(&tx, model.uid, 0, TokenType::Refresh).await?;

		tx.commit().await?;

		Ok(AccountInfo {
			account: model.into(),
			access_token,
			refresh_token,
		})
	}

	async fn auth(&self, token: &str) -> Result<AuthInfo, UserError> {
		let db = self.db();
		let tx = db.begin().await?;

		// find token record
		let model = token::Entity::find()
			.filter(token::Column::Token.eq(token))
			.filter(token::Column::Typ.ne(token::TokenTypeDef::Refresh))
			.one(&tx)
			.await?
			.ok_or_else(|| UserError::TokenInvalid)?;

		// check token

		let uid = model.uid;
		let profile_id = model.profile_id;

		let mut token_am: token::ActiveModel = model.clone().into();

		let token: Token = model.into();
		if token.is_expired() {
			return Err(UserError::TokenExpired);
		}

		let info = match token.typ {
			TokenType::Access => {
				// find account
				let account = account::Entity::find()
					.filter(account::Column::Uid.eq(uid))
					.one(&tx)
					.await?
					.ok_or_else(|| UserError::UserNotFound)?;

				// update
				let mut active_model: account::ActiveModel = account.into();
				active_model.last_login = ActiveValue::Set(Utc::now());

				let updated_account = active_model.update(&tx).await?;

				AuthInfo::Account(Account::from(updated_account))
			}
			TokenType::Session => {
				// find profile
				let profile = profile::Entity::find()
					.filter(profile::Column::AccountId.eq(uid))
					.filter(profile::Column::Id.eq(profile_id))
					.one(&tx)
					.await?
					.ok_or_else(|| UserError::ProfileNotFound)?;

				// renew session token
				token_am.expire = ActiveValue::Set(Utc::now() + token.typ.duration());
				token_am.update(&tx).await?;

				AuthInfo::Profile(Profile::from(profile))
			}
			_ => {
				return Err(UserError::TokenInvalid);
			}
		};

		tx.commit().await?;

		Ok(info)
	}

	async fn logout(&self, access_token: &str) -> Result<(), UserError> {
		let db = self.db();
		let tx = db.begin().await?;

		// find token record
		let account_model = verify_access_token(&tx, access_token).await?;
		let uid = account_model.uid;

		// remove all tokens under the same uid
		token::Entity::delete_many().filter(token::Column::Uid.eq(uid)).exec(&tx).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn delete_account(&self, username: &str, password: &str) -> Result<(), UserError> {
		let db = self.db();
		let tx = db.begin().await?;

		let model = account::Entity::find()
			.filter(account::Column::Name.eq(username))
			.one(&tx)
			.await?
			.ok_or_else(|| UserError::InvalidUsernameOrPassword)?;

		if !password.verify_password(&model.secret) {
			return Err(UserError::InvalidUsernameOrPassword);
		}

		let uid = model.uid;

		// remove all tokens under the same uid
		token::Entity::delete_many().filter(token::Column::Uid.eq(uid)).exec(&tx).await?;

		// remove all profiles under the same uid
		// TODO: remove all profile data
		profile::Entity::delete_many().filter(profile::Column::AccountId.eq(uid)).exec(&tx).await?;

		// remove the account
		model.delete(&tx).await?;

		tx.commit().await?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {

	use std::time::Duration;

	use emukc_db::entity::user::{self, token::TokenTypeDef};
	use emukc_model::codex::Codex;
	use prelude::DbConn;

	use super::*;

	async fn new_mock() -> (DbConn, Codex) {
		let db = emukc_db::prelude::new_mem_db().await.unwrap();
		let codex = Codex::default();
		return (db, codex);
	}

	#[tokio::test]
	async fn test_new_account() {
		let gameplay = new_mock().await;

		let username = "admin";
		let password = "abcd123";

		let result = gameplay.sign_up(username, password).await.unwrap();

		println!("{:?}", result);
	}

	#[tokio::test]
	async fn test_token_issue() {
		let gp = new_mock().await;

		let result = gp.sign_up("test", "1234567").await.unwrap();
		let uid = result.account.uid;

		let db = &gp.0;
		let tx = db.begin().await.unwrap();

		for _ in 0..3 {
			issue_token(&tx, uid, 0, TokenType::Access).await.unwrap();
		}

		let tokens = user::token::Entity::find()
			.filter(user::token::Column::Uid.eq(uid))
			.filter(user::token::Column::Typ.eq(TokenTypeDef::Access))
			.all(&tx)
			.await
			.unwrap();

		assert_eq!(tokens.len(), 1);
	}

	#[tokio::test]
	async fn test_login() {
		let gp = new_mock().await;
		let username = "test";
		let password = "1234567";

		let signup = gp.sign_up(username, password).await.unwrap();

		println!("{:?}", signup);

		tokio::time::sleep(Duration::from_secs(1)).await;

		let login = gp.sign_in(username, password).await.unwrap();

		println!("{:?}", login);

		assert_eq!(signup.account.uid, login.account.uid);

		let access_token = login.access_token.token;

		let auth = gp.auth(&access_token).await.unwrap();

		let account = match auth {
			AuthInfo::Account(account) => account,
			AuthInfo::Profile(profile) => panic!("expect account, got profile: {:?}", profile),
		};
		assert_eq!(account.uid, signup.account.uid);

		gp.logout(&access_token).await.unwrap();

		let auth = gp.auth(&access_token).await;

		assert!(auth.is_err());
	}
}
