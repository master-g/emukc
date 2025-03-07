/// Trait for converting a value to a version string.
pub trait IntoVersion {
	/// Converts the value to a version string.
	fn into_version(self) -> Option<String>;
}

impl IntoVersion for String {
	fn into_version(self) -> Option<String> {
		if self == "" || self == "1" {
			None
		} else {
			Some(self)
		}
	}
}

impl IntoVersion for &str {
	fn into_version(self) -> Option<String> {
		if self == "" || self == "1" {
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
		self
	}
}

impl IntoVersion for Option<&str> {
	fn into_version(self) -> Option<String> {
		self.map(|s| s.to_string())
	}
}

impl IntoVersion for Option<&String> {
	fn into_version(self) -> Option<String> {
		self.map(|s| s.to_string())
	}
}

impl IntoVersion for Option<i64> {
	fn into_version(self) -> Option<String> {
		self.map(|v| v.to_string())
	}
}

impl IntoVersion for Option<()> {
	fn into_version(self) -> Option<String> {
		None
	}
}

/// NoVersion represents a version that is not available.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct NoVersion;

impl IntoVersion for NoVersion {
	fn into_version(self) -> Option<String> {
		None
	}
}
