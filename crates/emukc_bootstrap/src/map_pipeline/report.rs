use std::fmt;

use serde::{Deserialize, Serialize};

/// Indicates how the stat.json source was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapCatalogStatSource {
    /// stat.json was downloaded from GitHub.
    Downloaded,
    /// stat.json was loaded from local cache.
    Cached,
    /// stat.json was unavailable (download failed, no cache).
    Unavailable,
}

/// Bootstrap-owned provenance for a finalized runtime `MapCatalog`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapCatalogBuildReport {
    /// Number of maps the observed drops and enemy fleets cover.
    pub label_overlay_map_count: usize,
    /// Number of maps present in the public overlay input.
    pub public_overlay_map_count: usize,
    /// Number of maps with stat.json data.
    pub stat_map_count: usize,
    /// How stat.json was obtained.
    pub stat_source: MapCatalogStatSource,
    /// Number of maps in the final assembled runtime catalog.
    pub output_map_count: usize,
    /// Number of label overlay items dropped during fan-out because their node label
    /// was absent from the target variant's cell set.
    pub fanout_rules_dropped: usize,
    /// Number of kcdata YAML files that failed to deserialize (skipped with a warning).
    pub kcdata_parse_errors: usize,
    /// Number of topology validation warnings emitted during assembly.
    pub topology_warnings: usize,
}

impl fmt::Display for MapCatalogBuildReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "map catalog build: {} maps output ({} observed, {} overlay, {} stat)",
            self.output_map_count,
            self.label_overlay_map_count,
            self.public_overlay_map_count,
            self.stat_map_count,
        )?;
        if self.fanout_rules_dropped > 0 {
            write!(f, "; fanout rules dropped: {}", self.fanout_rules_dropped)?;
        }
        if self.kcdata_parse_errors > 0 {
            write!(f, "; kcdata parse errors: {}", self.kcdata_parse_errors)?;
        }
        if self.topology_warnings > 0 {
            write!(f, "; topology warnings: {}", self.topology_warnings)?;
        }
        Ok(())
    }
}
