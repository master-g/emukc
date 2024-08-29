//! File cache entry from `KanColle` CDN.

use serde::{Deserialize, Serialize};

/// File cache entry from `KanColle` CDN.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[non_exhaustive]
pub struct KcFileEntry {
	/// Relative path to the file.
	/// for example:
	/// A file located at `http://kancolle-cdn/path/to/file.ext` will have a path of `path/to/file.ext`.
	pub path: String,

	/// MD5 hash of the file.
	pub md5: String,

	/// Version of the file.
	pub version: Option<String>,
}

impl KcFileEntry {
	/// Create a new `KcFileEntry`.
	pub fn new(path: &str, md5: &str, version: Option<&str>) -> Self {
		Self {
			path: Self::unify_path(path),
			md5: md5.to_string(),
			version: version.map(std::string::ToString::to_string),
		}
	}

	/// Create a new `KcFileEntry` from a model.
	pub fn from_model(path: String, md5: String, version: Option<String>) -> Self {
		Self {
			path,
			md5,
			version,
		}
	}

	fn unify_path(path: &str) -> String {
		path.replace('\\', "/").trim_start_matches('/').to_owned()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_unify_path() {
		assert_eq!(KcFileEntry::unify_path("path/to/file.ext"), "path/to/file.ext");
		assert_eq!(KcFileEntry::unify_path("/path/to/file.ext"), "path/to/file.ext");
		assert_eq!(KcFileEntry::unify_path("\\path\\to\\file.ext"), "path/to/file.ext");
		assert_eq!(KcFileEntry::unify_path("\\path/to/file.ext"), "path/to/file.ext");
	}

	#[test]
	fn test_serialize_deserialize() {
		let entry = KcFileEntry::new("path/to/file.ext", "md5", Some("version"));
		let json = serde_json::to_string(&entry).unwrap();
		let entry2: KcFileEntry = serde_json::from_str(&json).unwrap();
		assert_eq!(entry, entry2);
	}
}
