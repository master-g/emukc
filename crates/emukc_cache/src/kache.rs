//! Kache is for `KanColle` Cache, a simple cache system.

use std::{path::PathBuf, sync::Arc};

use emukc_crypto::md5_file_async;
use emukc_db::{entity::cache, sea_orm::*};
use emukc_model::cache::KcFileEntry;
use emukc_network::{client::new_reqwest_client, download};
use thiserror::Error;
use tokio::io::AsyncReadExt;

/// The chunk size for batch processing.
const CHUNK_SIZE: usize = 256;

/// Error type for `Kache`.
#[derive(Debug, Error)]
pub enum Error {
	/// Missing field error.
	#[error("missing field: {0}")]
	MissingField(String),

	/// File not found error.
	#[error("file not found: {0}")]
	FileNotFound(String),

	/// Invalid file error.
	#[error("invalid file: {0}")]
	InvalidFile(String),

	/// File expired error.
	#[error("file expired: {0}")]
	FileExpired(String),

	/// IO error.
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),

	/// Database error.
	#[error("database error: {0}")]
	Db(#[from] DbErr),

	/// Download error.
	#[error("download request builder error: {0}")]
	DownloadRequestBuilder(#[from] download::BuilderError),

	/// Download error.
	#[error("download error: {0}")]
	Download(#[from] download::DownloadError),

	/// Failed on all CDN.
	#[error("failed on all CDN")]
	FailedOnAllCdn,

	/// Reqwest error.
	#[error("reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),
}

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
	db: Arc<DbConn>,
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
	pub fn with_mods_root(mut self, mods_root: PathBuf) -> Self {
		self.mods_root = Some(mods_root);
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
	pub fn with_proxy(mut self, proxy: String) -> Self {
		self.proxy = Some(proxy);
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
	#[instrument]
	pub async fn get(&self, path: &str, version: Option<&str>) -> Result<tokio::fs::File, Error> {
		info!("🔍 {}, ver: {}", path, version.unwrap_or("n/a"));

		let f = match self.find_in_mods(path).await {
			Some(f) => f,
			None => match self.find_in_local_or_fetch_from_remote(path, version).await {
				Ok(file) => file,
				Err(e) => {
					error!("❗️ local_path:{}, err:{:?}", path, e);
					return Err(e);
				}
			},
		};

		info!("✅ {}, {}", path, version.unwrap_or("n/a"));

		Ok(f)
	}

	/// Export all records from the database.
	///
	/// Return a list of `KcFileEntry`.
	pub async fn export(&self) -> Result<Vec<KcFileEntry>, Error> {
		let db = &*self.db;
		let models = cache::Entity::find().all(db).await?;
		Ok(models.into_iter().map(Into::into).collect())
	}

	/// Import records to the database.
	///
	/// # Arguments
	///
	/// * `entries` - The list of `KcFileEntry`.
	///
	/// Return a tuple of `(new_entries, updated_entries)`.
	#[instrument(skip_all)]
	pub async fn import(&self, entries: &[KcFileEntry]) -> Result<(usize, usize), Error> {
		let db = &*self.db;

		trace!("importing {} entries", entries.len());
		let mut not_exists = vec![];
		let mut updates = vec![];

		let mut now = std::time::SystemTime::now();

		for entry in entries {
			let model =
				cache::Entity::find().filter(cache::Column::Path.eq(&entry.path)).one(db).await?;

			let Some(model) = model else {
				not_exists.push(entry);
				continue;
			};

			let old_entry: KcFileEntry = model.clone().into();
			if entry.version_cmp(&old_entry) == std::cmp::Ordering::Greater {
				let mut am: cache::ActiveModel = model.into();
				am.version = ActiveValue::Set(entry.version.clone());
				updates.push(am);
			}
		}

		trace!("filtering done, elapsed: {:?}", now.elapsed().unwrap());

		let updated = updates.len();
		debug!("{} entries need to be updated", updated);
		now = std::time::SystemTime::now();
		{
			let chunks = updates.chunks(CHUNK_SIZE);
			let mut processed = 0;
			for chunk in chunks {
				let txn = db.begin().await?;
				for am in chunk.iter().cloned() {
					am.update(&txn).await?;
				}
				txn.commit().await?;
				processed += chunk.len();
				debug!("processed {}/{}", processed, updated);
			}
		}
		trace!("updating done, elapsed: {:?}", now.elapsed().unwrap());

		let new_entries = not_exists.len();
		debug!("{} new entries", new_entries);
		now = std::time::SystemTime::now();
		{
			let chunks = not_exists.chunks(CHUNK_SIZE);
			let mut processed = 0;
			for chunk in chunks {
				let models = chunk
					.iter()
					.map(|entry| cache::ActiveModel {
						id: ActiveValue::NotSet,
						path: ActiveValue::Set(entry.path.clone()),
						md5: ActiveValue::Set(entry.md5.clone()),
						version: ActiveValue::Set(entry.version.clone()),
					})
					.collect::<Vec<_>>();
				cache::Entity::insert_many(models).exec(db).await?;

				processed += chunk.len();
				debug!("processed {}/{}", processed, new_entries);
			}
		}
		trace!("inserting done, elapsed: {:?}", now.elapsed().unwrap());

		Ok((new_entries, updated))
	}

	/// Find the file in the mods.
	/// Version will be ignored.
	#[instrument]
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

	/// Find the file in the local cache.
	///
	/// 1. load db record.
	/// 2. check if the file exists.
	/// 3. check if the checksum matched.
	/// 4. if not matched, return error.
	/// 5. if not found in db, and is a non-versioned file, insert db record.
	///
	/// # Arguments
	///
	/// * `rel_path` - The file's relative path.
	/// * `local_path` - The file's local path.
	/// * `version` - The file version.
	#[instrument(skip(self))]
	async fn find_in_local(
		&self,
		rel_path: &str,
		local_path: &PathBuf,
		version: Option<&str>,
	) -> Result<tokio::fs::File, Error> {
		let db = &*self.db;

		let model = cache::Entity::find().filter(cache::Column::Path.eq(rel_path)).one(db).await?;

		// find db entry
		if let Some(model) = model {
			if !local_path.exists() {
				trace!("file not found in cache, but found in db: {:?}", model);
				return Err(Error::FileNotFound(local_path.to_str().unwrap().to_owned()));
			}

			// check if version matched
			let v = model.version.filter(|v| !v.is_empty());
			if version != v.as_deref() {
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
		if version.is_none() && local_path.exists() {
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
	#[instrument]
	async fn fetch_from_url(
		&self,
		url: &str,
		rel_path: &str,
		local_path: &PathBuf,
		version: Option<&str>,
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

		let record = cache::Entity::find()
			.filter(cache::Column::Path.eq(rel_path))
			.filter(cache::Column::Version.eq(version.unwrap_or("")))
			.one(db)
			.await?;

		let mut model = cache::ActiveModel {
			id: ActiveValue::NotSet,
			path: ActiveValue::Set(rel_path.to_owned()),
			md5: ActiveValue::Set(md5),
			version: ActiveValue::Set(version.map(ToOwned::to_owned)),
		};

		if let Some(record) = record {
			model.id = ActiveValue::Unchanged(record.id);
		}

		model.save(db).await?;

		Ok(tokio::fs::File::open(local_path).await?)
	}

	/// Find the file in the local cache or fetch from the remote CDN.
	///
	/// # Arguments
	///
	/// * `path` - The file's relative path.
	/// * `version` - The file version.
	#[instrument]
	async fn find_in_local_or_fetch_from_remote(
		&self,
		path: &str,
		version: Option<&str>,
	) -> Result<tokio::fs::File, Error> {
		let local_path = self.cache_root.join(path);
		let str_ver = version.unwrap_or("n/a");

		match self.find_in_local(path, &local_path, version).await {
			Ok(file) => return Ok(file),
			Err(e) => match e {
				Error::FileNotFound(_) => {
					warn!("🤔 missing: {}, version: {}", path, str_ver);
				}
				Error::InvalidFile(_) => {
					warn!("❌ invalid: {}, version: {}", path, str_ver);
				}
				Error::FileExpired(_) => {
					warn!("🥀 expired: {}, version: {}", path, str_ver);
				}
				_ => return Err(e),
			},
		};

		self.fetch_from_remote(path, &local_path, version).await
	}

	async fn fetch_from_remote(
		&self,
		path: &str,
		local_path: &PathBuf,
		version: Option<&str>,
	) -> Result<tokio::fs::File, Error> {
		let cdn_list = if path.starts_with("gadget_html5") || path.starts_with("html") {
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
			let url = format!("{}/{}?version={}", cdn, remote_path, version.unwrap_or(""));
			info!("🛫 {}", url);

			match self.fetch_from_url(&url, path, &local_path, version).await {
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
	/// A valid cache file should not be a HTML file.
	///
	/// # Arguments
	///
	/// * `path` - The file path.
	async fn is_valid(path: &std::path::Path) -> bool {
		if !path.exists() || !path.is_file() {
			return false;
		}
		let Ok(mut file) = tokio::fs::File::open(path).await else {
			return false;
		};

		let mut buffer = [0; 1];
		if file.read_exact(&mut buffer).await.is_err() {
			return false;
		}

		buffer[0] != b'<'
	}

	/// Check all the cache files.
	///
	/// # Arguments
	///
	/// * `fix` - Whether to fix the invalid or missing files.
	pub async fn check_all(&self, fix: bool) -> Result<(usize, usize, usize), Error> {
		let mut invalid = 0;
		let mut missing = 0;

		let db = &*self.db;
		let models = cache::Entity::find().all(db).await.unwrap();
		let total = models.len();
		for model in models {
			trace!("checking: {:?}", model);
			let abs_path = self.cache_root.join(&model.path);
			if !abs_path.exists() {
				missing += 1;
				if fix {
					debug!("missing: {:?}", abs_path);
					let _ = self
						.fetch_from_remote(&model.path, &abs_path, model.version.as_deref())
						.await?;
				} else {
					warn!("missing file: {:?}", abs_path);
				}
				continue;
			}

			let md5 = md5_file_async(&abs_path).await?;
			if md5 != model.md5 {
				invalid += 1;
				if fix {
					debug!("invalid: {:?}", abs_path);
					let _ = self
						.fetch_from_remote(&model.path, &abs_path, model.version.as_deref())
						.await?;
				} else {
					warn!("invalid file: {:?}", abs_path);
				}
			}
		}

		Ok((total, invalid, missing))
	}
}
