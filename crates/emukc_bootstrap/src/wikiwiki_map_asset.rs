use std::{borrow::Cow, io, path::PathBuf};

use crate::assets::{RepoAssetSource, WIKIWIKI_MAP_CATALOG};

/// Source used to provide the runtime wikiwiki map catalog asset.
pub type RepoWikiwikiMapCatalogSource = RepoAssetSource;

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

/// Canonical repo-tracked label-space wikiwiki map catalog asset path.
pub fn repo_wikiwiki_map_catalog_path() -> PathBuf {
    WIKIWIKI_MAP_CATALOG.path()
}

/// Load the repo-tracked wikiwiki map catalog, falling back to the embedded asset.
pub fn load_repo_wikiwiki_map_catalog_asset() -> io::Result<RepoWikiwikiMapCatalogAsset> {
    let (source, raw_json) = WIKIWIKI_MAP_CATALOG.load()?;
    Ok(RepoWikiwikiMapCatalogAsset {
        source,
        raw_json,
    })
}

#[cfg(test)]
mod tests {
    use emukc_model::codex::map::RoutePredicate;

    use super::*;
    use crate::parser::wikiwiki_map::WikiwikiMapOverlayCatalog;

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
