//! Deal with account related stuff.

use emukc_crypto::PasswordCrypto;
use emukc_db::{
	entity::user::{account, token},
	sea_orm::{entity::*, query::*},
};
use emukc_model::user::{
	account::Account,
	token::{Token, TokenType},
};
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

	#[error("Invalid username or password.")]
	InvalidUsernameOrPassword,

	#[error("Token invalid.")]
	TokenInvalid,

	#[error("Token expired.")]
	TokenExpired,

	#[error("User not found.")]
	UserNotFound,

	#[error("Database error: {0}")]
	Db(#[from] emukc_db::sea_orm::DbErr),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginResult {
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
	) -> Result<LoginResult, AccountError> {
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
		};
		let model = model.insert(&tx).await?;

		// issue new tokens
		let access_token = Self::issue_token(&tx, model.uid, 0, TokenType::Access).await?;
		let refresh_token = Self::issue_token(&tx, model.uid, 0, TokenType::Refresh).await?;

		// final commit
		tx.commit().await?;

		Ok(LoginResult {
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
	pub async fn sign_in(
		&self,
		username: &str,
		password: &str,
	) -> Result<LoginResult, AccountError> {
		let db = &*self.db;
		let tx = db.begin().await?;

		let model =
			account::Entity::find().filter(account::Column::Name.eq(username)).one(&tx).await?;

		let Some(model) = model else {
			return Err(AccountError::InvalidUsernameOrPassword);
		};

		if !password.verify_password(&model.secret) {
			return Err(AccountError::InvalidUsernameOrPassword);
		}

		let mut active_model: account::ActiveModel = model.into();
		active_model.last_login = ActiveValue::Set(Utc::now());

		let model = active_model.update(&tx).await?;

		let access_token = Self::issue_token(&tx, model.uid, 0, TokenType::Access).await?;
		let refresh_token = Self::issue_token(&tx, model.uid, 0, TokenType::Refresh).await?;

		tx.commit().await?;

		Ok(LoginResult {
			account: model.into(),
			access_token,
			refresh_token,
		})
	}

	/// Authenticate with an access token.
	///
	/// # Arguments
	///
	/// * `access_token` - The access token to authenticate with.
	pub async fn auth(&self, access_token: &str) -> Result<Account, AccountError> {
		let db = &*self.db;
		let tx = db.begin().await?;

		// find token record
		let model = token::Entity::find()
			.filter(token::Column::Token.eq(access_token))
			.filter(token::Column::Typ.eq(token::TokenTypeDef::Access))
			.one(&tx)
			.await?;

		let Some(model) = model else {
			return Err(AccountError::TokenInvalid);
		};

		// check token

		let uid = model.uid;

		let token: Token = model.into();
		if token.is_expired() {
			return Err(AccountError::TokenExpired);
		}

		// find account
		let account = account::Entity::find().filter(account::Column::Uid.eq(uid)).one(&tx).await?;
		let Some(account) = account else {
			return Err(AccountError::UserNotFound);
		};

		// update
		let mut active_model: account::ActiveModel = account.into();
		active_model.last_login = ActiveValue::Set(Utc::now());

		let model = active_model.update(&tx).await?;

		tx.commit().await?;

		Ok(model.into())
	}

	/// Logout with an access token.
	///
	/// # Arguments
	///
	/// * `access_token` - The access token to logout with.
	pub async fn logout(&self, access_token: &str) -> Result<(), AccountError> {
		let db = &*self.db;
		let tx = db.begin().await?;

		// find token record
		let model = token::Entity::find()
			.filter(token::Column::Token.eq(access_token))
			.filter(token::Column::Typ.eq(token::TokenTypeDef::Access))
			.one(&tx)
			.await?;

		let Some(model) = model else {
			return Err(AccountError::TokenInvalid);
		};

		let uid = model.uid;

		let token: Token = model.into();
		if token.is_expired() {
			return Err(AccountError::TokenExpired);
		}

		// remove all tokens under the same uid
		token::Entity::delete_many().filter(token::Column::Uid.eq(uid)).exec(&tx).await?;

		tx.commit().await?;

		Ok(())
	}

	async fn issue_token<C>(
		c: &C,
		uid: i64,
		profile_id: i64,
		typ: TokenType,
	) -> Result<Token, AccountError>
	where
		C: ConnectionTrait,
	{
		let token = match typ {
			TokenType::Access => Token::issue_access(uid),
			TokenType::Refresh => Token::issue_refresh(uid),
			TokenType::Session => Token::issue_session(uid, profile_id),
		};

		let db_token_type = token::TokenTypeDef::from(typ);

		// find the old token
		let record = token::Entity::find()
			.filter(token::Column::Uid.eq(uid))
			.filter(token::Column::ProfileId.eq(profile_id))
			.filter(token::Column::Typ.eq(db_token_type))
			.one(c)
			.await?;

		if let Some(record) = record {
			// remove the old token
			record.delete(c).await?;
		}

		// insert the new token
		token::ActiveModel {
			id: ActiveValue::NotSet,
			uid: ActiveValue::Set(uid),
			profile_id: ActiveValue::Set(profile_id),
			typ: ActiveValue::Set(typ.into()),
			token: ActiveValue::Set(token.token.clone()),
			expire: ActiveValue::Set(token.expire),
		}
		.save(c)
		.await?;

		Ok(token)
	}
}

#[cfg(test)]
mod tests {

	use std::time::Duration;

	use emukc_db::entity::user::{self, token::TokenTypeDef};

	use super::*;

	#[tokio::test]
	async fn test_new_account() {
		let gameplay = Gameplay::new_mock().await;

		let username = "admin";
		let password = "abcd123";

		let result = gameplay.sign_up(username, password).await.unwrap();

		println!("{:?}", result);
	}

	#[tokio::test]
	async fn test_token_issue() {
		let gp = Gameplay::new_mock().await;

		let result = gp.sign_up("test", "1234567").await.unwrap();
		let uid = result.account.uid;

		let db = gp.db;
		let tx = db.begin().await.unwrap();

		for _ in 0..3 {
			Gameplay::issue_token(&tx, uid, 0, TokenType::Access).await.unwrap();
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
		let gp = Gameplay::new_mock().await;
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

		assert_eq!(auth.uid, signup.account.uid);

		gp.logout(&access_token).await.unwrap();

		let auth = gp.auth(&access_token).await;

		assert!(auth.is_err());
	}
}
