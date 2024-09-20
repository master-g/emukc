//! Configuration module for the emukc binary.

use std::{
	path::{Path, PathBuf},
	sync::OnceLock,
};

use config::{Config, FileFormat};
use serde::{Deserialize, Serialize};

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
	pub async fn load(path: impl AsRef<str>) -> Result<Self, Box<dyn std::error::Error>> {
		let source = config::File::new(path.as_ref(), FileFormat::Toml);
		let cfg = Config::builder().add_source(source).build()?;
		let mut cfg = cfg.try_deserialize::<AppConfig>()?;

		if cfg.workspace_root.is_relative() {
			let cfg_path = Path::new(path.as_ref()).parent().unwrap();
			// join cfg_dif and workspace_root
			let workspace_root = cfg_path.join(&cfg.workspace_root);
			cfg.workspace_root = workspace_root;
		}

		CFG.set(cfg.clone()).unwrap();

		Ok(cfg)
	}
}
