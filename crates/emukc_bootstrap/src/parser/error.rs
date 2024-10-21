//! Possible errors that can occur during parsing.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	#[error("yaml error: {0}")]
	Yaml(#[from] serde_yaml::Error),

	#[error("json error: {0}")]
	Json(#[from] serde_json::Error),

	#[error("key missing")]
	KeyMissing,
}
