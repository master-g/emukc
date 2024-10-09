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

	#[error("Invalid manifest ID: {0}")]
	ManifestNotFound(i64),

	#[error("Capacity exceeded: {0}")]
	CapacityExceeded(i64),

	#[error("Failed to create new ship: {0}")]
	ShipCreationFailed(i64),

	#[error("Codex error: {0}")]
	Codex(#[from] emukc_model::codex::CodexError),

	#[error("Entry not found: {0}")]
	EntryNotFound(String),
}
