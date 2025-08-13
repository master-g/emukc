//! Kache is for `KanColle` Cache, a simple cache system.

use std::path::PathBuf;
use std::sync::Arc;

use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use redb::{Database, ReadableDatabase, TableDefinition};
use tokio::io::AsyncReadExt;

use emukc_network::{client::new_reqwest_client, download, reqwest};

use crate::{
	error::Error,
	opt::GetOption,
	unified_rel_path,
	ver::{IntoVersion, cmp_version},
};

pub(crate) const KACHE_TABLE: TableDefinition<&str, Option<&str>> =
	TableDefinition::new("kache_entry");

/// The `Kache` struct is the main struct for the `KanColle` CDN file cache utilities.
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct Kache {
	/// Root directory for the cache.
	cache_root: PathBuf,

	/// Root directory for the mods.
	mods_root: Option<PathBuf>,

	/// CDN URLs for downloading gadgets.
	gadgets_cdn: Vec<String>,

	/// CDN URLs for downloading game res contents.
	content_cdn: Vec<String>,

	/// rewuest client for downloading files.
	client: reqwest::Client,

	/// Database for storing cache entries.
	pub(crate) db: Arc<Database>,
}

/// The `Builder` struct is the builder for the `Kache` struct.
#[derive(Debug, Clone, Default)]
pub struct Builder {
	cache_root: Option<PathBuf>,
	mods_root: Option<PathBuf>,
	gadgets_cdn: Vec<String>,
	content_cdn: Vec<String>,
	proxy: Option<String>,
	ua: Option<String>,
	db_path: Option<String>,
}

impl Builder {
	/// Create a new `Builder` instance.
	pub fn new() -> Self {
		Self::default()
	}

	/// Set the cache root directory.
	pub fn with_cache_root(mut self, cache_root: PathBuf) -> Self {
		self.cache_root = Some(cache_root);
		self
	}

	/// Set the mods root directory.
	pub fn with_mods_root(mut self, mods_root: Option<PathBuf>) -> Self {
		self.mods_root = mods_root;
		self
	}

	/// Set the gadgets CDN URLs.
	pub fn with_gadgets_cdn(mut self, cdn: String) -> Self {
		self.gadgets_cdn.push(cdn);
		self
	}

	/// Set the CDN URLs.
	pub fn with_gadgets_cdns(mut self, cdns: Vec<String>) -> Self {
		self.gadgets_cdn.extend(cdns);
		self
	}

	/// Set the content CDN URLs.
	pub fn with_content_cdn(mut self, cdn: String) -> Self {
		self.content_cdn.push(cdn);
		self
	}

	/// Set the content CDN URLs.
	pub fn with_content_cdns(mut self, cdns: Vec<String>) -> Self {
		self.content_cdn.extend(cdns);
		self
	}

	/// Set the proxy for downloading files.
	pub fn with_proxy(mut self, proxy: Option<String>) -> Self {
		self.proxy = proxy;
		self
	}

	/// Set the custom UA for downloading files.
	pub fn with_user_agent(mut self, ua: String) -> Self {
		self.ua = Some(ua);
		self
	}

	/// Set the database path.
	/// This will use specified path for the database rather than the default path.
	pub fn with_db_path(mut self, path: String) -> Self {
		self.db_path = Some(path);
		self
	}

	/// Build the `Kache` struct.
	#[allow(clippy::result_large_err)]
	pub fn build(self) -> Result<Kache, Error> {
		let cache_root = self.cache_root.ok_or(Error::MissingField("cache_root".to_owned()))?;
		let gadgets_cdn = if self.gadgets_cdn.is_empty() {
			return Err(Error::MissingField("gadgets_cdn".to_owned()));
		} else {
			self.gadgets_cdn
		};
		let content_cdn = if self.content_cdn.is_empty() {
			return Err(Error::MissingField("content_cdn".to_owned()));
		} else {
			self.content_cdn
		};

		let db_path = if let Some(db_path) = self.db_path {
			PathBuf::from(db_path)
		} else {
			cache_root.join("kache.redb")
		};

		let client = new_reqwest_client(self.proxy.as_deref(), self.ua.as_deref())?;
		debug!("proxy: {}", self.proxy.as_deref().unwrap_or("none"));

		let db = Database::create(&db_path)?;
		let write_txn = db.begin_write()?;
		{
			let _ = write_txn.open_table(KACHE_TABLE)?;
		}
		write_txn.commit()?;

		let db = Arc::new(db);

		Ok(Kache {
			cache_root,
			mods_root: self.mods_root,
			gadgets_cdn,
			content_cdn,
			client,
			db,
		})
	}
}

impl Kache {
	/// Create a new `Builder` instance.
	pub fn builder() -> Builder {
		Builder::new()
	}

	/// Get file from the cache.
	///
	/// 1. if the file exists in the cache, return the file path. else 2.
	/// 2. download the file from the CDN and save it to the cache.
	/// 3. update the database.
	/// 4. return the file.
	///
	/// # Arguments
	///
	/// * `path` - The file's relative path.
	/// * `version` - The file version.
	pub async fn get<V>(&self, path: &str, version: V) -> Result<tokio::fs::File, Error>
	where
		V: IntoVersion,
	{
		self.get_with_opt(path, version, &GetOption::new()).await
	}

	/// Get file from the cache.
	///
	/// # Arguments
	///
	/// * `path` - The file's relative path.
	/// * `version` - The file version.
	/// * `opt` - The options for the get operation.
	pub async fn get_with_opt<V>(
		&self,
		path: &str,
		version: V,
		opt: &GetOption,
	) -> Result<tokio::fs::File, Error>
	where
		V: IntoVersion,
	{
		let v = version.into_version().unwrap_or_default();

		let log_tail = if v.is_empty() {
			path.to_string()
		} else {
			format!("{path}, version: {v}")
		};

		if v.is_empty() {
			debug!("🔍 {log_tail}");
		} else {
			debug!("🔍 {log_tail}");
		}

		if opt.enable_mod && self.mods_root.is_some() {
			if let Some(f) = self.find_in_mods(path).await {
				debug!("✅ {log_tail}");
				return Ok(f);
			}
		}

		let local_path = self.cache_root.join(path);
		if opt.enable_local {
			match self.find_in_local(path, &local_path, &v).await {
				Ok(file) => {
					debug!("✅ {log_tail}");
					return Ok(file);
				}
				Err(e) => match e {
					Error::FileNotFound(_) => {
						warn!("🤔 missing: {log_tail}");
					}
					Error::InvalidFile(_) => {
						warn!("❌ invalid: {log_tail}");
					}
					Error::FileExpired(_) => {
						warn!("🥀 expired: {log_tail}");
					}
					Error::InvalidFileVersion(_) => {
						warn!("🎃 version rollback: {log_tail}");
					}
					_ => {
						error!("❗️ local_path:{}, err:{:?}", path, e);
						return Err(e);
					}
				},
			};
		}

		if opt.enable_remote {
			return self
				.fetch_from_remote(path, &local_path, &v, opt.enable_shuffle)
				.await
				.inspect(|_file| {
					debug!("✅ {log_tail}");
				});
		}

		Err(Error::FileNotFound(path.to_owned()))
	}

	/// Check if the file exists on the remote CDN.
	pub async fn exists_on_remote<V>(&self, path: &str, ver: V) -> Result<bool, Error>
	where
		V: IntoVersion,
	{
		let v = if let Some(v) = ver.into_version() {
			v
		} else {
			"".to_string()
		};
		let cdn_list = if path.starts_with("gadget_html5")
			|| path.starts_with("html")
			|| path.contains("world.html")
		{
			&self.gadgets_cdn
		} else {
			&self.content_cdn
		};

		if cdn_list.is_empty() {
			error!("🚫 no available cdn");
			return Err(Error::MissingField("cdn_list".to_owned()));
		}

		for cdn in cdn_list {
			let cdn = cdn.trim_end_matches('/');
			let cdn = if cdn.starts_with("http") {
				cdn.to_string()
			} else {
				format!("http://{cdn}")
			};
			let remote_path = path.trim_start_matches('/');
			let ver = if v.is_empty() {
				"".to_string()
			} else {
				format!("?version={v}")
			};
			let url = format!("{cdn}/{remote_path}{ver}");
			trace!("🔍 {}", &url);

			match self.client.head(&url).send().await {
				Ok(resp) => match resp.status() {
					reqwest::StatusCode::OK => {
						trace!("✅ {}", &url);
						return Ok(true);
					}
					reqwest::StatusCode::NOT_FOUND => {
						trace!("🚫 not found: {}", &url);
						return Ok(false);
					}
					_ => {
						trace!("💥 url:{}, status:{:?}", url, resp.status());
					}
				},
				Err(e) => {
					trace!("💥 url:{}, error:{:?}", url, e);
				}
			}
		}

		Err(Error::FailedOnAllCdn)
	}

	/// Find the file in the mods.
	/// Version will be ignored.
	#[instrument(skip(self))]
	async fn find_in_mods(&self, path: &str) -> Option<tokio::fs::File> {
		let mod_path = self.mods_root.as_ref()?;

		let local_path = mod_path.join(path);
		if local_path.exists() {
			info!("👻 mod found {:?}", local_path);
			Some(tokio::fs::File::open(local_path).await.unwrap())
		} else {
			// check for wildcard
			let ext = local_path.extension()?.to_str()?;
			let parent_dir = local_path.parent()?;
			let wildcard_file = parent_dir.join(format!("wildcard.{ext}"));

			if wildcard_file.exists() {
				info!("👻 wildcard mod found {wildcard_file:?}");
				Some(tokio::fs::File::open(wildcard_file).await.unwrap())
			} else {
				None
			}
		}
	}

	async fn find_in_local(
		&self,
		rel_path: &str,
		local_path: &PathBuf,
		version: &str,
	) -> Result<tokio::fs::File, Error> {
		if !local_path.exists() {
			trace!("file not found");
			return Err(Error::FileNotFound(local_path.to_str().unwrap().to_owned()));
		}

		if !Self::is_valid(local_path).await {
			trace!("invalid file");
			return Err(Error::InvalidFile(local_path.to_str().unwrap().to_owned()));
		}

		let metadata = local_path.metadata()?;
		if !metadata.is_file() {
			trace!("not a file");
			return Err(Error::FileNotFound(local_path.to_str().unwrap().to_owned()));
		}

		let read_txn = self.db.begin_read()?;
		let table = read_txn.open_table(KACHE_TABLE)?;

		let rel_path = unified_rel_path(rel_path);

		let entry = table.get(rel_path.as_str())?;

		if let Some(ver) = entry {
			let v = ver.value();
			return match cmp_version(v, version) {
				std::cmp::Ordering::Less => {
					trace!("the required version is newer than the local version");
					Err(Error::FileExpired(rel_path.to_owned()))
				}
				std::cmp::Ordering::Equal => {
					trace!("the required version is equal to the local version");
					Ok(tokio::fs::File::open(local_path).await?)
				}
				std::cmp::Ordering::Greater => {
					trace!(
						"{} the required version {} is older than the local version {:?}",
						rel_path, version, v
					);
					Err(Error::InvalidFileVersion(rel_path.to_owned()))
					// return Ok(tokio::fs::File::open(local_path).await?);
				}
			};
		}

		let required_version = version.into_version();
		if required_version.is_none() {
			trace!("the required version is none");
			Ok(tokio::fs::File::open(local_path).await?)
		} else {
			trace!("required version has no record in local");
			Err(Error::FileExpired(rel_path))
		}
	}

	/// Fetch the file from CDN url.
	///
	/// 1. download the file from the remote CDN.
	/// 2. save the file to the local cache.
	/// 3. update the database.
	/// 4. return the file.
	///
	/// # Arguments
	///
	/// * `url` - The remote CDN URL.
	/// * `rel_path` - The file's relative path.
	/// * `local_path` - The file's local path.
	/// * `version` - The file version.
	#[instrument(skip(self))]
	async fn fetch_from_url(
		&self,
		url: &str,
		rel_path: &str,
		local_path: &PathBuf,
		version: &str,
	) -> Result<tokio::fs::File, Error> {
		download::Request::builder()
			.url(url)
			.save_as(local_path)
			.overwrite(true)
			.build()?
			.execute(Some(self.client.clone()))
			.await?;

		if !Self::is_valid(local_path).await {
			error!("invalid file: {:?}", local_path);
			return Err(Error::InvalidFile(local_path.to_str().unwrap().to_owned()));
		}

		let rel_path = unified_rel_path(rel_path);
		let write_txn = self.db.begin_write()?;
		{
			let mut table = write_txn.open_table(KACHE_TABLE)?;
			let v = version.into_version();
			table.insert(rel_path.as_str(), v.as_deref())?;
		}
		write_txn.commit()?;

		Ok(tokio::fs::File::open(local_path).await?)
	}

	async fn fetch_from_remote(
		&self,
		path: &str,
		local_path: &PathBuf,
		version: &str,
		shuffle: bool,
	) -> Result<tokio::fs::File, Error> {
		let cdn_list = if path.starts_with("gadget_html5")
			|| path.starts_with("html")
			|| path.contains("world.html")
		{
			&self.gadgets_cdn
		} else {
			&self.content_cdn
		};

		if cdn_list.is_empty() {
			error!("🚫 no available cdn");
			return Err(Error::MissingField("cdn_list".to_owned()));
		}

		let cdn_list = if shuffle {
			let mut cloned = cdn_list.clone();
			let mut rng = SmallRng::from_os_rng();
			cloned.shuffle(&mut rng);
			cloned
		} else {
			cdn_list.to_vec()
		};

		for cdn in cdn_list {
			let cdn = cdn.trim_end_matches('/');
			let cdn = if cdn.starts_with("http") {
				cdn.to_string()
			} else {
				format!("http://{cdn}")
			};
			let remote_path = path.trim_start_matches('/');
			let ver = if version.is_empty() {
				"".to_string()
			} else {
				format!("?version={version}")
			};
			let url = format!("{cdn}/{remote_path}{ver}");
			info!("🛫 {}", url);

			match self.fetch_from_url(&url, path, local_path, version).await {
				Ok(f) => {
					info!("🛬 {}", url);
					return Ok(f);
				}
				Err(e) => {
					error!("💥 url:{}, err:{:?}", url, e);
				}
			}
		}

		error!("🚫 all cdn failed for {}", path);

		Err(Error::FailedOnAllCdn)
	}

	/// Check if the file is valid.
	///
	/// # Arguments
	///
	/// * `path` - The file path.
	async fn is_valid(path: &std::path::Path) -> bool {
		if !path.exists() || !path.is_file() {
			trace!("File does not exist or is not a file: {:?}", path);
			return false;
		}

		// Check if the file is a HTML file.
		if path.extension().is_some_and(|ext| ext == "html") {
			trace!("File is a HTML file: {:?}", path);
			return true;
		}

		trace!("File is not a HTML file: {:?}", path);

		let Ok(mut file) = tokio::fs::File::open(path).await else {
			trace!("Failed to open file: {:?}", path);
			return false;
		};

		// check if file is empty
		let Ok(metadata) = file.metadata().await else {
			return false;
		};
		if metadata.len() == 0 {
			return true;
		}

		let mut buffer = [0; 1];
		if file.read_exact(&mut buffer).await.is_err() {
			trace!("Failed to read file: {:?}", path);
			return false;
		}

		buffer[0] != b'<'
	}
}
