//! Configuration module for the emukc binary.

use std::{
	path::{Path, PathBuf},
	sync::OnceLock,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use config::{Config, FileFormat};

/// The global configuration
pub static CFG: OnceLock<AppConfig> = OnceLock::new();

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
	/// The root directory of the application workspace
	pub workspace_root: PathBuf,

	/// The root directory of the KC cache
	pub cache_root: PathBuf,

	/// The root directory of the KC mods
	pub mods_root: PathBuf,

	/// addr to bind the server to
	pub bind: std::net::SocketAddr,

	/// Proxy server to use for HTTP requests
	pub proxy: Option<String>,

	/// The URL to the gadgets CDN
	pub gadgets_cdn: Vec<String>,

	/// The URL to the game files CDN
	pub game_cdn: Vec<String>,
}

impl AppConfig {
	/// Load the configuration from a file
	///
	/// # Arguments
	///
	/// * `path` - The path to the configuration file
	pub fn load(path: impl AsRef<str>) -> Result<Self> {
		let source = config::File::new(path.as_ref(), FileFormat::Toml);
		let cfg = Config::builder().add_source(source).build()?;
		let mut cfg = cfg.try_deserialize::<AppConfig>()?;

		if cfg.workspace_root.is_relative() {
			let cfg_path = Path::new(path.as_ref()).parent().unwrap();
			// join cfg_dif and workspace_root
			let workspace_root = cfg_path.join(&cfg.workspace_root);
			cfg.workspace_root = if workspace_root.is_absolute() {
				workspace_root
			} else {
				workspace_root.canonicalize()?
			};

			// join cfg_dif and cache_root
			let cache_root = cfg_path.join(&cfg.cache_root);
			cfg.cache_root = if cache_root.is_absolute() {
				cache_root
			} else {
				cache_root.canonicalize()?
			};

			// join cfg_dif and mods_root
			let mods_root = cfg_path.join(&cfg.mods_root);
			cfg.mods_root = if mods_root.is_absolute() {
				mods_root
			} else {
				mods_root.canonicalize()?
			};
		}

		CFG.set(cfg.clone()).unwrap();

		Ok(cfg)
	}

	/// Get the path to the template files directory
	pub fn temp_root(&self) -> Result<PathBuf> {
		self.ensure_dir("temp")
	}

	/// Get the path to the log files directory
	pub fn log_root(&self) -> Result<PathBuf> {
		self.ensure_dir("logs")
	}

	/// Get the path to the codex files directory
	pub fn codex_root(&self) -> Result<PathBuf> {
		self.ensure_dir("codex")
	}

	#[allow(unused)]
	/// Get the path to a directory relative to the workspace root
	fn dir(&self, dir: impl AsRef<Path>) -> PathBuf {
		self.workspace_root.join(dir)
	}

	#[allow(unused)]
	/// Ensure that a directory exists
	fn ensure_dir(&self, dir: impl AsRef<Path>) -> Result<PathBuf> {
		let dir = self.dir(dir);
		if !dir.exists() {
			std::fs::create_dir_all(&dir)?;
		}
		Ok(dir)
	}
}
