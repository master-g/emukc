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
use thiserror::Error;

use crate::gameplay::HasContext;

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

	#[error("Profile not found.")]
	ProfileNotFound,

	#[error("Database error: {0}")]
	Db(#[from] emukc_db::sea_orm::DbErr),
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartGameInfo {
	pub profile: Profile,
	pub session: Token,
}

/// A trait for account related gameplay.
#[async_trait]
pub trait AccountGameplay {
	/// Create a new account.
	///
	/// # Arguments
	///
	/// * `username` - The username of the new account.
	/// * `password` - The password of the new account.
	async fn sign_up(&self, username: &str, password: &str) -> Result<AccountInfo, AccountError>;

	/// Sign in with username and password.
	///
	/// # Arguments
	///
	/// * `username` - The username of the account.
	/// * `password` - The password of the account.
	async fn sign_in(&self, username: &str, password: &str) -> Result<AccountInfo, AccountError>;

	/// Start a game session.
	///
	/// # Arguments
	///
	/// * `access_token` - The access token of the account.
	/// * `profile_id` - The profile ID to start the game with.
	async fn start_game(
		&self,
		access_token: &str,
		profile_id: i64,
	) -> Result<StartGameInfo, AccountError>;

	/// Authenticate with an access token.
	///
	/// # Arguments
	///
	/// * `token` - The token to authenticate with, usually an access token, or game session token.
	async fn auth(&self, token: &str) -> Result<AuthInfo, AccountError>;

	/// Logout with an access token.
	///
	/// # Arguments
	///
	/// * `access_token` - The access token to logout with.
	async fn logout(&self, access_token: &str) -> Result<(), AccountError>;

	/// Remove an account and all its data.
	///
	/// # Arguments
	///
	/// * `username` - The username of the account.
	/// * `password` - The password of the account.
	async fn delete_account(&self, username: &str, password: &str) -> Result<(), AccountError>;
}

#[async_trait]
impl<T: HasContext + ?Sized> AccountGameplay for T {
	async fn sign_up(&self, username: &str, password: &str) -> Result<AccountInfo, AccountError> {
		let db = self.db();

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
	async fn sign_in(&self, username: &str, password: &str) -> Result<AccountInfo, AccountError> {
		let db = self.db();
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

		let access_token = issue_token(&tx, model.uid, 0, TokenType::Access).await?;
		let refresh_token = issue_token(&tx, model.uid, 0, TokenType::Refresh).await?;

		tx.commit().await?;

		Ok(AccountInfo {
			account: model.into(),
			access_token,
			refresh_token,
		})
	}

	async fn start_game(
		&self,
		access_token: &str,
		profile_id: i64,
	) -> Result<StartGameInfo, AccountError> {
		let db = self.db();
		let tx = db.begin().await?;

		// verify access token
		let account_model = token::Entity::find()
			.filter(token::Column::Token.eq(access_token))
			.filter(token::Column::Typ.eq(token::TokenTypeDef::Access))
			.one(&tx)
			.await?;

		let Some(account_model) = account_model else {
			return Err(AccountError::TokenInvalid);
		};

		let token: Token = account_model.into();
		if token.is_expired() {
			return Err(AccountError::TokenExpired);
		}

		let uid = token.uid;

		// find profile
		let profile_model = profile::Entity::find()
			.filter(profile::Column::AccountId.eq(uid))
			.filter(profile::Column::Id.eq(profile_id))
			.one(&tx)
			.await?;

		let Some(profile_model) = profile_model else {
			return Err(AccountError::ProfileNotFound);
		};

		let token = issue_token(&tx, uid, profile_id, TokenType::Session).await?;

		tx.commit().await?;

		Ok(StartGameInfo {
			profile: profile_model.into(),
			session: token,
		})
	}

	async fn auth(&self, token: &str) -> Result<AuthInfo, AccountError> {
		let db = self.db();
		let tx = db.begin().await?;

		// find token record
		let model = token::Entity::find()
			.filter(token::Column::Token.eq(token))
			.filter(token::Column::Typ.ne(token::TokenTypeDef::Refresh))
			.one(&tx)
			.await?;

		let Some(model) = model else {
			return Err(AccountError::TokenInvalid);
		};

		// check token

		let uid = model.uid;
		let profile_id = model.profile_id;

		let token: Token = model.into();
		if token.is_expired() {
			return Err(AccountError::TokenExpired);
		}

		let info = match token.typ {
			TokenType::Access => {
				// find account
				let account =
					account::Entity::find().filter(account::Column::Uid.eq(uid)).one(&tx).await?;
				let Some(account) = account else {
					return Err(AccountError::UserNotFound);
				};

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
					.await?;

				let Some(profile) = profile else {
					return Err(AccountError::ProfileNotFound);
				};

				AuthInfo::Profile(Profile::from(profile))
			}
			_ => {
				return Err(AccountError::TokenInvalid);
			}
		};

		tx.commit().await?;

		Ok(info)
	}

	async fn logout(&self, access_token: &str) -> Result<(), AccountError> {
		let db = self.db();
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

	async fn delete_account(&self, username: &str, password: &str) -> Result<(), AccountError> {
		let db = self.db();
		let tx = db.begin().await?;

		let model =
			account::Entity::find().filter(account::Column::Name.eq(username)).one(&tx).await?;

		let Some(model) = model else {
			return Err(AccountError::InvalidUsernameOrPassword);
		};

		if !password.verify_password(&model.secret) {
			return Err(AccountError::InvalidUsernameOrPassword);
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
