//! Deal with account related stuff.

use emukc_db::sea_orm::TransactionTrait;
use thiserror::Error;

use crate::prelude::Gameplay;

#[derive(Debug, Error)]
pub enum AccountError {
	#[error("The username is already taken.")]
	UsernameTaken,

	#[error("Database error: {0}")]
	Db(#[from] emukc_db::sea_orm::DbErr),
}

impl Gameplay {
	/// Create a new account.
	///
	/// # Arguments
	///
	/// * `username` - The username of the new account.
	/// * `password` - The password of the new account.
	pub async fn new_account(&self, username: &str, password: &str) -> Result<(), AccountError> {
		let db = &*self.db;

		let tx = db.begin().await?;

		Ok(())
	}
}
