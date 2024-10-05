//! Gameplay errors

use emukc_db::sea_orm;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameplayError {
	#[error("Profile not found: {0}")]
	ProfileNotFound(i64),

	#[error("Database error: {0}")]
	Db(#[from] sea_orm::error::DbErr),

	#[error("Invalid material category: {0}")]
	InvalidMaterialCategory(i64),
}
