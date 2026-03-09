/// Trait for converting a value to a version string.
pub trait IntoVersion {
	/// Converts the value to a version string.
	fn into_version(self) -> Option<String>;
}

impl IntoVersion for String {
	fn into_version(self) -> Option<String> {
		if self.is_empty() || self == "1" {
			None
		} else {
			Some(self)
		}
	}
}

impl IntoVersion for &str {
	fn into_version(self) -> Option<String> {
		if self.is_empty() || self == "1" {
			None
		} else {
			Some(self.to_string())
		}
	}
}

impl IntoVersion for &String {
	fn into_version(self) -> Option<String> {
		if self.as_str() == "" || self.as_str() == "1" {
			None
		} else {
			Some(self.as_str().to_string())
		}
	}
}

impl IntoVersion for i64 {
	fn into_version(self) -> Option<String> {
		if self == 0 || self == 1 {
			None
		} else {
			Some(self.to_string())
		}
	}
}

impl IntoVersion for Option<String> {
	fn into_version(self) -> Option<String> {
		self.and_then(IntoVersion::into_version)
	}
}

impl IntoVersion for Option<&str> {
	fn into_version(self) -> Option<String> {
		self.and_then(IntoVersion::into_version)
	}
}

impl IntoVersion for Option<&String> {
	fn into_version(self) -> Option<String> {
		self.and_then(IntoVersion::into_version)
	}
}

impl IntoVersion for Option<i64> {
	fn into_version(self) -> Option<String> {
		self.and_then(IntoVersion::into_version)
	}
}

impl IntoVersion for Option<()> {
	fn into_version(self) -> Option<String> {
		None
	}
}

/// `NoVersion` represents a version that is not available.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct NoVersion;

impl IntoVersion for NoVersion {
	fn into_version(self) -> Option<String> {
		None
	}
}

/// Compare two version strings.
///
/// # Arguments
///
/// * `a` - The first version string.
/// * `b` - The second version string.
///
/// # Returns
///
/// * `Ordering::Less` if `a` is less than `b`.
pub fn cmp_version<A, B>(a: A, b: B) -> std::cmp::Ordering
where
	A: IntoVersion,
	B: IntoVersion,
{
	let ver_a = a.into_version();
	let ver_b = b.into_version();

	match (ver_a.as_deref(), ver_b.as_deref()) {
		(None, Some("")) | (Some(""), None) | (None, None) => std::cmp::Ordering::Equal,
		(Some(_), None) => std::cmp::Ordering::Greater,
		(None, Some(_)) => std::cmp::Ordering::Less,
		(Some(a), Some(b)) => ver_str_cmp(a, b),
	}
}

fn ver_str_cmp(a: &str, b: &str) -> std::cmp::Ordering {
	if a == b {
		return std::cmp::Ordering::Equal;
	}
	if a.is_empty() {
		return std::cmp::Ordering::Less;
	}
	if b.is_empty() {
		return std::cmp::Ordering::Greater;
	}

	let a_parts: Vec<i32> = a.split('.').filter_map(|s| s.parse().ok()).collect();
	let b_parts: Vec<i32> = b.split('.').filter_map(|s| s.parse().ok()).collect();

	if a_parts.is_empty() || b_parts.is_empty() {
		return a.cmp(b);
	}

	for (a_part, b_part) in a_parts.iter().zip(b_parts.iter()) {
		match a_part.cmp(b_part) {
			std::cmp::Ordering::Equal => continue,
			ord => return ord,
		}
	}

	a_parts.len().cmp(&b_parts.len())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_version_comparison() {
		assert_eq!(cmp_version("1.2.3", "1.2.4"), std::cmp::Ordering::Less);
		assert_eq!(cmp_version("1.2.4", "1.2.3"), std::cmp::Ordering::Greater);
		assert_eq!(cmp_version("1.2.3", "1.2.3"), std::cmp::Ordering::Equal);
	}

	#[test]
	fn test_empty_version() {
		assert_eq!(cmp_version("", "1.0.0"), std::cmp::Ordering::Less);
		assert_eq!(cmp_version("1.0.0", ""), std::cmp::Ordering::Greater);
		assert_eq!(cmp_version("", ""), std::cmp::Ordering::Equal);
	}

	#[test]
	fn test_invalid_version_fallback() {
		// When both versions are invalid, fallback to string comparison
		assert_eq!(cmp_version("abc", "def"), std::cmp::Ordering::Less);
		// "1.2.3" parses to [1,2,3], "1.2.abc" parses to [1,2], so [1,2,3] > [1,2]
		assert_eq!(cmp_version("1.2.3", "1.2.abc"), std::cmp::Ordering::Greater);
	}

	#[test]
	fn test_different_length_versions() {
		assert_eq!(cmp_version("1.2", "1.2.0"), std::cmp::Ordering::Less);
		assert_eq!(cmp_version("1.2.0", "1.2"), std::cmp::Ordering::Greater);
	}
}
