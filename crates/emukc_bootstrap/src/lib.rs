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

#[macro_use]
extern crate tracing;

mod battle_rules;
mod db;
mod download;
mod make_list;
mod map_overlay;
mod map_pipeline;
mod parser;
mod populate;
mod progress;
mod real_map_start_asset;
mod res;
mod wikiwiki_map_asset;
/// Manual wikiwiki map download helpers used by examples and one-off tooling.
pub mod wikiwiki_map_download;

/// The `emukc_bootstrap` crate prelude.
pub mod prelude {
    pub use crate::battle_rules::{
        BattleIncidentReport, BattleIncidentTriggerMatch, BattleKnowledgeAssetSources,
        BattleKnowledgeAssets, BattleModuleIndexAsset, BattleModuleKnowledge,
        BattleProtocolFieldRule, BattleProtocolFieldsAsset, BattleResourceRule,
        BattleResourceRulesAsset, BattleSlotResourceTrigger, BattleSlotResourceTriggersAsset,
        BattleValidationFinding, BattleValidationFindingKind, BattleValidationReport,
        BattleValidationSeverity, ExpectedBattleResource, RepoBattleKnowledgeSource,
        analyze_day_battle_incident, load_repo_battle_knowledge_assets,
        repo_battle_module_index_path, repo_battle_protocol_fields_path,
        repo_battle_resource_rules_path, repo_battle_slot_resource_triggers_path,
        validate_day_battle_response,
    };
    pub use crate::db::{DbBootstrapError, prepare};
    pub use crate::download::BootstrapDownloadError;
    pub use crate::download::download_all;
    pub use crate::download::download_web_assets;
    pub use crate::make_list::{
        CacheListBuildDiagnostics, CacheListComparisonReport, CacheListItem, CacheListMakeStrategy,
        CacheListPathBuildOutput, CacheListPathPrefixCount, apply_candidate_build_diagnostics,
        build_cache_list_items, build_cache_list_items_with_manifest_path,
        build_cache_list_items_with_rules_path, build_cache_list_path_output_with_rules_path,
        build_cache_list_paths, build_cache_list_paths_with_manifest_path,
        build_cache_list_paths_with_rules_path, compare_cache_list_path_sets, config::GreedyConfig,
        errors::CacheListMakingError, make as make_cache_list,
    };
    pub use crate::map_overlay::{
        MapOverlayAcceptedRecord, MapOverlayBuildError, MapOverlayBuildOutput,
        MapOverlayBuildReport, MapOverlayRejectedRecord,
        build_public_map_catalog_overlay_from_embedded_real_map_start_assets,
        build_public_map_catalog_overlay_from_response_saver_dir,
        repo_public_map_catalog_overlay_path,
    };
    pub use crate::map_pipeline::{
        MapCatalogBuildReport, MapCatalogStatSource, MapCatalogWikiwikiSource,
        build_final_map_catalog, build_final_map_catalog_from_repo_assets,
        build_final_map_catalog_from_repo_assets_with_report, build_final_map_catalog_with_overlay,
        build_final_map_catalog_with_report,
    };
    pub use crate::parser::{
        WikiwikiLabelOverlay, WikiwikiMapOverlayCatalog, WikiwikiMapOverlayDefinition,
        parse_partial_codex, parse_wikiwiki_map, parse_wikiwiki_map_debug,
    };
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
