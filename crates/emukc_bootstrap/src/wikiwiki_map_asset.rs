use std::{
	borrow::Cow,
	fs, io,
	path::{Path, PathBuf},
};

const EMBEDDED_WIKIWIKI_MAP_CATALOG_JSON: &str =
	include_str!("../assets/wikiwiki_map_catalog.json");

/// Source used to provide the runtime wikiwiki map catalog asset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoWikiwikiMapCatalogSource {
	/// Loaded from the repo-tracked JSON file on disk.
	Filesystem(PathBuf),
	/// Loaded from the compile-time embedded fallback JSON.
	Embedded,
}

/// Raw repo/embedded wikiwiki map catalog asset payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoWikiwikiMapCatalogAsset {
	/// Where the raw JSON came from.
	pub source: RepoWikiwikiMapCatalogSource,
	raw_json: Cow<'static, str>,
}

impl RepoWikiwikiMapCatalogAsset {
	/// Return the raw catalog JSON contents.
	pub fn raw_json(&self) -> &str {
		&self.raw_json
	}
}

/// Canonical repo-tracked normalized wikiwiki map catalog asset path.
pub fn repo_wikiwiki_map_catalog_path() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/wikiwiki_map_catalog.json")
}

/// Load the repo-tracked wikiwiki map catalog, falling back to the embedded asset.
pub fn load_repo_wikiwiki_map_catalog_asset() -> io::Result<RepoWikiwikiMapCatalogAsset> {
	load_repo_wikiwiki_map_catalog_asset_from(&repo_wikiwiki_map_catalog_path())
}

fn load_repo_wikiwiki_map_catalog_asset_from(
	path: &Path,
) -> io::Result<RepoWikiwikiMapCatalogAsset> {
	match fs::read_to_string(path) {
		Ok(raw_json) => Ok(RepoWikiwikiMapCatalogAsset {
			source: RepoWikiwikiMapCatalogSource::Filesystem(path.to_path_buf()),
			raw_json: Cow::Owned(raw_json),
		}),
		Err(source) if source.kind() == io::ErrorKind::NotFound => {
			Ok(RepoWikiwikiMapCatalogAsset {
				source: RepoWikiwikiMapCatalogSource::Embedded,
				raw_json: Cow::Borrowed(EMBEDDED_WIKIWIKI_MAP_CATALOG_JSON),
			})
		}
		Err(source) => Err(source),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn load_repo_wikiwiki_map_catalog_asset_prefers_filesystem_contents() {
		let root = tempfile::tempdir().unwrap();
		let asset_path = root.path().join("wikiwiki_map_catalog.json");
		std::fs::write(&asset_path, r#"{"maps":{"11":{"map_id":11}}}"#).unwrap();

		let asset = load_repo_wikiwiki_map_catalog_asset_from(&asset_path).unwrap();

		assert_eq!(asset.source, RepoWikiwikiMapCatalogSource::Filesystem(asset_path));
		assert_eq!(asset.raw_json(), r#"{"maps":{"11":{"map_id":11}}}"#);
	}

	#[test]
	fn load_repo_wikiwiki_map_catalog_asset_falls_back_to_embedded_catalog() {
		let asset =
			load_repo_wikiwiki_map_catalog_asset_from(Path::new("/definitely/missing.json"))
				.unwrap();

		assert_eq!(asset.source, RepoWikiwikiMapCatalogSource::Embedded);
		assert!(asset.raw_json().contains("\"maps\""));
	}
}
