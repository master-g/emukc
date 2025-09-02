//! Gameplay errors

use emukc_db::sea_orm;
use emukc_model::thirdparty::reward::RewardError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GameplayError {
	#[error("Profile not found: {0}")]
	ProfileNotFound(i64),

	#[error("Database error: {0}")]
	Db(#[from] sea_orm::error::DbErr),

	#[error("Wrong type: {0}")]
	WrongType(String),

	#[error("Invalid manifest ID: {0}")]
	ManifestNotFound(i64),

	#[error("Bad manifest: {0}")]
	BadManifest(String),

	#[error("Capacity exceeded: {0}")]
	CapacityExceeded(i64),

	#[error("Failed to create new ship: {0}")]
	ShipCreationFailed(i64),

	#[error("Codex error: {0}")]
	Codex(#[from] emukc_model::codex::CodexError),

	#[error("Entry not found: {0}")]
	EntryNotFound(String),

	#[error("Insufficient item: {0}")]
	Insufficient(String),

	#[error("JSON error: {0}")]
	Json(#[from] serde_json::Error),

	#[error("Quest status invalid: {0}")]
	QuestStatusInvalid(String),

	#[error(transparent)]
	Reward(#[from] RewardError),
}
