use serde::{Deserialize, Serialize};

/// Indicates where the wikiwiki input for the final map catalog came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapCatalogWikiwikiSource {
	/// No wikiwiki catalog was provided; the build used `kc_data` plus overlays only.
	None,
	/// The caller supplied a normalized wikiwiki catalog explicitly.
	Provided,
	/// The repo-tracked wikiwiki catalog was loaded from the filesystem.
	Filesystem,
	/// The embedded fallback wikiwiki catalog was used.
	Embedded,
}

/// Bootstrap-owned provenance for a finalized runtime `MapCatalog`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapCatalogBuildReport {
	/// The wikiwiki source used during assembly.
	pub wikiwiki_source: MapCatalogWikiwikiSource,
	/// Number of maps present in the wikiwiki input, if any.
	pub wikiwiki_map_count: usize,
	/// Number of maps present in the `kc_data` structural input.
	pub kcdata_map_count: usize,
	/// Number of maps present in the public overlay input.
	pub public_overlay_map_count: usize,
	/// Number of maps in the final assembled runtime catalog.
	pub output_map_count: usize,
}
