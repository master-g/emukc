//! Possible errors that can occur during parsing.

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	#[error("yaml error: {0}")]
	Yaml(#[from] serde_yaml::Error),

	#[error("json error: {0}")]
	Json(#[from] serde_json::Error),

	#[error("key missing: {0}")]
	KeyMissing(String),

	#[error("parse int error: {0}")]
	IntParse(String),

	#[error("generic error: {0}")]
	Generic(String),

	#[error("I/O error at {}: {source}", path.display())]
	IoAt {
		path: PathBuf,
		#[source]
		source: std::io::Error,
	},

	#[error("yaml error at {}: {source}", path.display())]
	YamlAt {
		path: PathBuf,
		#[source]
		source: serde_yaml::Error,
	},

	#[error("json error at {}: {source}", path.display())]
	JsonAt {
		path: PathBuf,
		#[source]
		source: serde_json::Error,
	},
}

impl ParseError {
	pub fn io_at(path: impl AsRef<Path>, source: std::io::Error) -> Self {
		Self::IoAt {
			path: path.as_ref().to_path_buf(),
			source,
		}
	}

	pub fn yaml_at(path: impl AsRef<Path>, source: serde_yaml::Error) -> Self {
		Self::YamlAt {
			path: path.as_ref().to_path_buf(),
			source,
		}
	}

	pub fn json_at(path: impl AsRef<Path>, source: serde_json::Error) -> Self {
		Self::JsonAt {
			path: path.as_ref().to_path_buf(),
			source,
		}
	}
}
