use crate::{IntoVersion, Kache, KacheError};

/// Represents options for the `get` method of `Kache`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GetOption {
	/// Whether to enable the local cache.
	pub enable_local: bool,

	/// Whether to enable the remote cache.
	pub enable_remote: bool,

	/// Whether to enable the module cache.
	pub enable_mod: bool,

	/// Whether to enable the version check.
	pub enable_version_check: bool,
}

impl Default for GetOption {
	fn default() -> Self {
		Self {
			enable_local: true,
			enable_remote: true,
			enable_mod: true,
			enable_version_check: true,
		}
	}
}

impl GetOption {
	/// Creates a new `GetOption` with default values.
	pub fn new() -> Self {
		Self::default()
	}

	/// Creates a new `GetOption` with API mocking options.
	pub fn new_api_mocking() -> Self {
		Self {
			enable_local: false,
			enable_remote: false,
			enable_mod: true,
			enable_version_check: false,
		}
	}

	/// Disables the local cache.
	pub fn disable_local(mut self) -> Self {
		self.enable_local = false;
		self
	}

	/// Disables the remote cache.
	pub fn disable_remote(mut self) -> Self {
		self.enable_remote = false;
		self
	}

	/// Disables the module cache.
	pub fn disable_mod(mut self) -> Self {
		self.enable_mod = false;
		self
	}

	/// Disables the version check.
	pub fn disable_version_check(mut self) -> Self {
		self.enable_version_check = false;
		self
	}

	/// Executes the `get` method of `Kache` with the given options.
	///
	/// # Arguments
	/// * `cache` - The cache to use.
	/// * `rel_path` - The relative path to the file.
	/// * `version` - The version of the file.
	pub async fn get(
		self,
		cache: &Kache,
		rel_path: &str,
		version: impl IntoVersion,
	) -> Result<tokio::fs::File, KacheError> {
		cache.get_with_opt(rel_path, version, &self).await
	}
}
