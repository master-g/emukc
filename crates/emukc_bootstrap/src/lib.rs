//! The `emukc_bootstrap` crate provides the bootstrap utilities for the `EmuKC` project.
//!
//! This crate handles:
//! - Downloading and parsing game data (ships, items, quests, etc.)
//! - Preparing the database with initial data
//! - Creating cache lists for game assets
//! - Populating the database with parsed data
//!
//! The bootstrap process ensures that all necessary game data and assets are available
//! for the `EmuKC` server to function properly.

#![doc(html_favicon_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![doc(html_logo_url = "http://203.104.209.71/kcs2/resources/useitem/card_/090.png")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[allow(unused_imports)]
#[macro_use]
extern crate tracing;

mod db;
mod download;
mod make_list;
mod map_overlay;
mod map_pipeline;
mod parser;
mod populate;
mod real_map_start_asset;
mod res;
mod wikiwiki_map_asset;
/// Manual wikiwiki map download helpers used by examples and one-off tooling.
pub mod wikiwiki_map_download;

/// The `emukc_bootstrap` crate prelude.
pub mod prelude {
	pub use crate::db::{DbBootstrapError, prepare};
	pub use crate::download::BootstrapDownloadError;
	pub use crate::download::download_all;
	pub use crate::make_list::{
		CacheListMakeStrategy, config::GreedyConfig, errors::CacheListMakingError,
		make as make_cache_list,
	};
	pub use crate::map_overlay::{
		MapOverlayAcceptedRecord, MapOverlayBuildError, MapOverlayBuildOutput,
		MapOverlayBuildReport, MapOverlayRejectedRecord,
		build_public_map_catalog_overlay_from_embedded_real_map_start_assets,
		build_public_map_catalog_overlay_from_response_saver_dir,
		repo_public_map_catalog_overlay_path,
	};
	pub use crate::map_pipeline::{
		MapCatalogBuildReport, MapCatalogWikiwikiSource, build_final_map_catalog,
		build_final_map_catalog_from_repo_assets,
		build_final_map_catalog_from_repo_assets_with_report, build_final_map_catalog_with_report,
	};
	pub use crate::parser::{parse_partial_codex, parse_wikiwiki_map, parse_wikiwiki_map_debug};
	pub use crate::populate::populate;
	pub use crate::real_map_start_asset::{EMBEDDED_REAL_MAP_START_ASSETS, RealMapStartAsset};
	pub use crate::wikiwiki_map_asset::{
		RepoWikiwikiMapCatalogAsset, RepoWikiwikiMapCatalogSource,
		load_repo_wikiwiki_map_catalog_asset, repo_wikiwiki_map_catalog_path,
	};
	pub use crate::wikiwiki_map_download::{
		WikiwikiMapDownloadOptions, WikiwikiMapDownloadStats, download_wikiwiki_map,
		download_wikiwiki_map_with_options, wikiwiki_map_page_url,
	};
}
