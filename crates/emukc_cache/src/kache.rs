//! Kache is for `KanColle` Cache, a simple cache system.

use std::{path::PathBuf, sync::Arc};

use emukc_crypto::md5_file_async;
use emukc_db::{entity::cache, sea_orm::*};
use emukc_network::{client::new_reqwest_client, download, reqwest};
use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use tokio::io::AsyncReadExt;

use crate::{error::Error, opt::GetOption, ver::IntoVersion};

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

	/// Database connection.
	pub(crate) db: Arc<DbConn>,
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
	db: Option<Arc<DbConn>>,
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

	/// Set the database connection.
	pub fn with_db(mut self, db: Arc<DbConn>) -> Self {
		self.db = Some(db);
		self
	}

	/// Build the `Kache` struct.
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
		let db = self.db.ok_or(Error::MissingField("db".to_owned()))?;
		let client = new_reqwest_client(self.proxy.as_deref(), self.ua.as_deref())?;
		debug!("proxy: {}", self.proxy.as_deref().unwrap_or("none"));
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
			match self.find_in_local(path, &local_path, &v, opt.enable_checksum).await {
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
				format!("http://{}", cdn)
			};
			let remote_path = path.trim_start_matches('/');
			let ver = if v.is_empty() {
				"".to_string()
			} else {
				format!("?version={v}")
			};
			let url = format!("{}/{}{}", cdn, remote_path, ver);
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
			let wildcard_file = parent_dir.join(format!("wildcard.{}", ext));

			if wildcard_file.exists() {
				info!("👻 wildcard mod found {:?}", wildcard_file);
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
		enable_checksum: bool,
	) -> Result<tokio::fs::File, Error> {
		if !enable_checksum {
			if !local_path.exists() {
				trace!("file not found in cache");
				return Err(Error::FileNotFound(local_path.to_str().unwrap().to_owned()));
			}
			return Ok(tokio::fs::File::open(local_path).await?);
		}

		if !local_path.exists() {
			trace!("file not found");
			return Err(Error::FileNotFound(local_path.to_str().unwrap().to_owned()));
		}

		let metadata = local_path.metadata()?;
		if !metadata.is_file() {
			trace!("not a file");
			return Err(Error::FileNotFound(local_path.to_str().unwrap().to_owned()));
		}

		if metadata.len() == 0 {
			trace!("file is empty");
			return Err(Error::FileNotFound(local_path.to_str().unwrap().to_owned()));
		}

		let db = &*self.db;

		let query = cache::Entity::find().filter(cache::Column::Path.eq(rel_path));
		let query = if version.is_empty() {
			query
		} else {
			query.filter(cache::Column::Version.eq(Some(version)))
		};
		let model = query.one(db).await?;

		// find db entry
		if let Some(model) = model {
			// check if version matched
			let v = model.version.unwrap_or_default();
			if version != v {
				trace!("version not matched: {:?} != {:?}", version, v);
				return Err(Error::FileExpired(rel_path.to_owned()));
			}

			// check if checksum matched
			let md5 = md5_file_async(local_path).await?;
			if md5 != model.md5 {
				trace!("checksum not matched: {:?} != {:?}", md5, model.md5);
				// file expired
				return Err(Error::FileExpired(rel_path.to_owned()));
			}

			return Ok(tokio::fs::File::open(local_path).await?);
		}

		// not found in db
		if version.is_empty() && local_path.exists() {
			// non-versioned file found
			if !Self::is_valid(local_path).await {
				// invalid file
				return Err(Error::InvalidFile(rel_path.to_owned()));
			}

			// calculate md5
			let md5 = md5_file_async(local_path).await?;

			// insert db
			let model = cache::ActiveModel {
				id: ActiveValue::NotSet,
				path: ActiveValue::Set(rel_path.to_owned()),
				md5: ActiveValue::Set(md5),
				version: ActiveValue::Set(None),
			};
			model.insert(db).await?;

			return Ok(tokio::fs::File::open(local_path).await?);
		}

		Err(Error::FileNotFound(rel_path.to_owned()))
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

		let md5 = md5_file_async(local_path).await?;
		let db = &*self.db;
		let tx = db.begin().await?;

		let query = cache::Entity::find().filter(cache::Column::Path.eq(rel_path));
		let query = if version.is_empty() {
			query
		} else {
			query.filter(cache::Column::Version.eq(Some(version)))
		};
		let record = query.one(&tx).await?;

		let mut model = cache::ActiveModel {
			id: ActiveValue::NotSet,
			path: ActiveValue::Set(rel_path.to_owned()),
			md5: ActiveValue::Set(md5),
			version: ActiveValue::Set(if version.is_empty() {
				None
			} else {
				Some(version.to_owned())
			}),
		};

		if let Some(record) = record {
			model.id = ActiveValue::Unchanged(record.id);
		}

		model.save(&tx).await?;

		tx.commit().await?;

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
				format!("http://{}", cdn)
			};
			let remote_path = path.trim_start_matches('/');
			let ver = if version.is_empty() {
				"".to_string()
			} else {
				format!("?version={version}")
			};
			let url = format!("{}/{}{}", cdn, remote_path, ver);
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
