use serde::{Deserialize, Serialize};

/// Indicates where the wikiwiki input for the final map catalog came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapCatalogWikiwikiSource {
    /// No wikiwiki catalog was provided; the build used overlays only.
    None,
    /// The caller supplied a normalized wikiwiki catalog explicitly.
    Provided,
    /// The repo-tracked wikiwiki catalog was loaded from the filesystem.
    Filesystem,
    /// The embedded fallback wikiwiki catalog was used.
    Embedded,
}

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
    /// The wikiwiki source used during assembly.
    pub wikiwiki_source: MapCatalogWikiwikiSource,
    /// Number of maps present in the wikiwiki input, if any.
    pub wikiwiki_map_count: usize,
    /// Number of maps present in the public overlay input.
    pub public_overlay_map_count: usize,
    /// Number of maps with stat.json data.
    pub stat_map_count: usize,
    /// How stat.json was obtained.
    pub stat_source: MapCatalogStatSource,
    /// Number of maps in the final assembled runtime catalog.
    pub output_map_count: usize,
    /// Number of wikiwiki routing rules dropped during fan-out because their
    /// `from_cell_no` or `to_cell_no` was absent from the target variant's cell set.
    pub fanout_rules_dropped: usize,
}
