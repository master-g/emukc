//! Every checked-in asset under `assets/`, listed once.
//!
//! The table decides each asset's repo path, whether the crate carries an
//! embedded copy, and what `battle drift-check` fingerprints (all of them).
//! Loading prefers the file on disk and falls back to the embedded copy, so a
//! binary run away from the source tree still has what it embedded.
//!
//! `real_map_start_data/` is not here: those captures are build and test
//! inputs, not synced assets, and are embedded by file name where used.

use std::{
    borrow::Cow,
    fs, io,
    path::{Path, PathBuf},
};

/// Where an asset's contents came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoAssetSource {
    /// Read from the repo-tracked file.
    Filesystem(PathBuf),
    /// The copy compiled into the crate.
    Embedded,
}

/// One checked-in asset: `assets/<name>.json`.
#[derive(Debug)]
pub struct RepoAsset {
    /// Logical name; also the file stem and the drift-check key.
    pub name: &'static str,
    embedded: Option<&'static str>,
}

macro_rules! asset {
    ($name:literal) => {
        RepoAsset {
            name: $name,
            embedded: None,
        }
    };
    ($name:literal, embedded) => {
        RepoAsset {
            name: $name,
            embedded: Some(include_str!(concat!("../assets/", $name, ".json"))),
        }
    };
}

pub(crate) const BATTLE_PROTOCOL_FIELDS: RepoAsset = asset!("battle_protocol_fields", embedded);
pub(crate) const BATTLE_RESOURCE_RULES: RepoAsset = asset!("battle_resource_rules", embedded);
pub(crate) const BATTLE_MODULE_INDEX: RepoAsset = asset!("battle_module_index", embedded);
pub(crate) const BATTLE_SLOT_RESOURCE_TRIGGERS: RepoAsset =
    asset!("battle_slot_resource_triggers", embedded);
pub(crate) const BATTLE_ATTACK_TYPE_ACCEPTANCE: RepoAsset =
    asset!("battle_attack_type_acceptance", embedded);
pub(crate) const WIKIWIKI_MAP_CATALOG: RepoAsset = asset!("wikiwiki_map_catalog", embedded);
pub(crate) const PUBLIC_MAP_CATALOG_OVERLAYS: RepoAsset =
    asset!("public_map_catalog_overlays", embedded);
pub(crate) const MAP_SHIP_DROPS: RepoAsset = asset!("map_ship_drops", embedded);
pub(crate) const CACHE_RULES: RepoAsset = asset!("cache_rules", embedded);

/// Every checked-in asset. The cache-list inputs without an embedded copy are
/// still found next to whatever rules file a caller names (see
/// `make_list::manifest::loader`); they are listed here for their repo path
/// and drift-check coverage.
pub const REPO_ASSETS: &[RepoAsset] = &[
    BATTLE_PROTOCOL_FIELDS,
    BATTLE_RESOURCE_RULES,
    BATTLE_MODULE_INDEX,
    BATTLE_SLOT_RESOURCE_TRIGGERS,
    BATTLE_ATTACK_TYPE_ACCEPTANCE,
    WIKIWIKI_MAP_CATALOG,
    PUBLIC_MAP_CATALOG_OVERLAYS,
    MAP_SHIP_DROPS,
    CACHE_RULES,
    asset!("resource_manifest"),
    asset!("resource_categories"),
    asset!("resource_id_sets"),
    asset!("audio_resources"),
    asset!("ui_resources"),
    asset!("resource_templates"),
];

impl RepoAsset {
    /// The repo-tracked file.
    pub fn path(&self) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets").join(format!("{}.json", self.name))
    }

    /// The compiled-in copy, if this asset has one.
    pub(crate) fn embedded(&self) -> Option<&'static str> {
        self.embedded
    }

    /// Read the repo file, falling back to the embedded copy when it is gone.
    pub fn load(&self) -> io::Result<(RepoAssetSource, Cow<'static, str>)> {
        self.load_from(&self.path())
    }

    /// [`load`](Self::load) with the file looked up at `path` instead.
    pub(crate) fn load_from(
        &self,
        path: &Path,
    ) -> io::Result<(RepoAssetSource, Cow<'static, str>)> {
        match fs::read_to_string(path) {
            Ok(raw) => Ok((RepoAssetSource::Filesystem(path.to_path_buf()), Cow::Owned(raw))),
            Err(err) if err.kind() == io::ErrorKind::NotFound => match self.embedded {
                Some(embedded) => Ok((RepoAssetSource::Embedded, Cow::Borrowed(embedded))),
                None => Err(err),
            },
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every listed asset exists in the repo, and an embedded copy is exactly
    /// the file it was compiled from.
    #[test]
    fn every_asset_is_on_disk_and_its_embedded_copy_matches() {
        for asset in REPO_ASSETS {
            let on_disk = fs::read_to_string(asset.path())
                .unwrap_or_else(|err| panic!("{}: {err}", asset.path().display()));
            if let Some(embedded) = asset.embedded() {
                assert_eq!(embedded, on_disk, "{} embedded copy is stale", asset.name);
            }
        }
    }

    #[test]
    fn asset_names_are_unique() {
        let mut names = REPO_ASSETS.iter().map(|asset| asset.name).collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), REPO_ASSETS.len());
    }

    #[test]
    fn a_present_file_wins_and_a_missing_one_falls_back_to_the_embedded_copy() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("wikiwiki_map_catalog.json");
        fs::write(&path, r#"{"maps":{}}"#).unwrap();

        let (source, raw) = WIKIWIKI_MAP_CATALOG.load_from(&path).unwrap();
        assert_eq!(source, RepoAssetSource::Filesystem(path));
        assert_eq!(raw, r#"{"maps":{}}"#);

        let (source, raw) =
            WIKIWIKI_MAP_CATALOG.load_from(Path::new("/definitely/missing.json")).unwrap();
        assert_eq!(source, RepoAssetSource::Embedded);
        assert!(raw.contains("\"maps\""));
    }

    #[test]
    fn a_missing_file_without_an_embedded_copy_is_an_error() {
        let err = REPO_ASSETS
            .iter()
            .find(|asset| asset.embedded().is_none())
            .unwrap()
            .load_from(Path::new("/definitely/missing.json"))
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
