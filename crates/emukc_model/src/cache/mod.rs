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

	/// Compare the version of two `KcFileEntry`.
	pub fn version_cmp(&self, other: &Self) -> std::cmp::Ordering {
		match (self.version.as_deref(), other.version.as_deref()) {
			(None, Some("")) | (Some(""), None) | (None, None) => std::cmp::Ordering::Equal,
			(Some(_), None) => std::cmp::Ordering::Greater,
			(None, Some(_)) => std::cmp::Ordering::Less,
			(Some(a), Some(b)) => Self::ver_str_cmp(a, b),
		}
	}

	/// Compare the version of two `KcFileEntry` by string.
	///
	/// # Arguments
	///
	/// * `a` - The first version string.
	/// * `b` - The second version string.
	pub fn ver_str_cmp(a: &str, b: &str) -> std::cmp::Ordering {
		if a == b {
			return std::cmp::Ordering::Equal;
		}
		if a.is_empty() {
			return std::cmp::Ordering::Less;
		}
		if b.is_empty() {
			return std::cmp::Ordering::Greater;
		}

		let a_parts: Vec<i32> = a.split('.').map(|s| s.parse().unwrap()).collect();
		let b_parts: Vec<i32> = b.split('.').map(|s| s.parse().unwrap()).collect();
		for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
			match a_part.cmp(b_part) {
				std::cmp::Ordering::Equal => continue,
				ord => return ord,
			}
		}

		a_parts.len().cmp(&b_parts.len())
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

	#[test]
	fn test_ver_cmp() {
		{
			let ver0 = "1.0.0";
			let ver1 = "1.0.1";
			assert_eq!(KcFileEntry::ver_str_cmp(ver0, ver1), std::cmp::Ordering::Less);
		}
		{
			let ver0 = "4.9.8";
			let ver1 = "5";
			assert_eq!(KcFileEntry::ver_str_cmp(ver0, ver1), std::cmp::Ordering::Less);
		}
		{
			let ver0 = "2.0.0";
			let ver1 = "10.0.0";
			assert_eq!(KcFileEntry::ver_str_cmp(ver0, ver1), std::cmp::Ordering::Less);
		}
		{
			let ver0 = "5.0.199";
			let ver1 = "5.1.0";
			assert_eq!(KcFileEntry::ver_str_cmp(ver0, ver1), std::cmp::Ordering::Less);
		}
		{
			let ver0 = "7.0.99.433";
			let ver1 = "7.1.199.3";
			assert_eq!(KcFileEntry::ver_str_cmp(ver0, ver1), std::cmp::Ordering::Less);
		}
	}
}
