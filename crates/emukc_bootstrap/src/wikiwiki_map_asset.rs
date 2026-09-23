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
    use emukc_model::codex::map::RoutePredicate;

    use super::*;
    use crate::parser::wikiwiki_map::WikiwikiMapOverlayCatalog;

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

    #[test]
    fn repo_asset_limits_route_history_rules_to_known_normal_maps() {
        let asset = load_repo_wikiwiki_map_catalog_asset().unwrap();
        let catalog = serde_json::from_str::<WikiwikiMapOverlayCatalog>(asset.raw_json()).unwrap();
        let mut visited_rules = Vec::new();

        for definition in catalog.maps.values() {
            for (variant_key, variant) in &definition.variants {
                for rule in &variant.routing_rules {
                    match &rule.predicate {
                        RoutePredicate::VisitedNodeLabel {
                            node_labels,
                            visited,
                        } => {
                            visited_rules.push((
                                definition.map_id,
                                variant_key.clone(),
                                rule.from_label.clone(),
                                rule.to_label.clone(),
                                *visited,
                                node_labels.clone(),
                            ));
                        }
                        RoutePredicate::VisitedNode {
                            cell_nos,
                            ..
                        } => {
                            panic!(
                                "label-space asset carries cell numbers on map {} variant `{}`: {cell_nos:?}",
                                definition.map_id, variant_key,
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        visited_rules.sort();
        // Guardrail: if this list changes, re-audit whether sortie-wide visited-node history
        // remains sufficient or if we need a first-class direct arrival-edge predicate.
        let rule = |map_id, from: &str, to: &str, label: &str| {
            (map_id, String::new(), from.to_string(), to.to_string(), true, vec![label.to_string()])
        };
        assert_eq!(
            visited_rules,
            vec![
                rule(45, "K", "M", "E"),
                rule(55, "M", "O", "N"),
                rule(55, "N", "O", "M"),
                rule(56, "C1", "C2", "A"),
                rule(56, "Q2", "T", "P"),
                rule(74, "J", "K", "D"),
            ]
        );
    }
}
