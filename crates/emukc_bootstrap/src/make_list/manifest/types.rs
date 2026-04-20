use serde::{Deserialize, Serialize};

/// The expected manifest version.
pub(crate) const MANIFEST_VERSION: i64 = 1;

/// The resource manifest root.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceManifest {
    /// Schema version.
    pub version: i64,
    /// ISO 8601 generation timestamp.
    pub generated_at: String,
    /// Summary statistics.
    #[serde(default)]
    pub summary: ResourceManifestSummary,
    /// Resource entries.
    pub entries: Vec<ResourceManifestEntry>,
}

/// Manifest summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceManifestSummary {
    #[serde(default)]
    pub total_entries: i64,
    #[serde(default)]
    pub ship_entry_count: i64,
    #[serde(default)]
    pub slotitem_entry_count: i64,
    #[serde(default)]
    pub texture_provider_entry_count: i64,
    #[serde(default)]
    pub explicit_path_entry_count: i64,
    #[serde(default)]
    pub total_explicit_paths: i64,
    #[serde(default)]
    pub modules_covered: i64,
}

/// A single resource manifest entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceManifestEntry {
    /// Entry kind.
    pub kind: ManifestEntryKind,
    /// Source expression (e.g. "resources.getShip").
    #[serde(default)]
    pub source: String,
    /// Target type (e.g. "full", "card").
    #[serde(default)]
    pub target_type: String,
    /// Ship MST ID source expression (ship entries only).
    #[serde(default)]
    pub ship_mst_id_source: Option<String>,
    /// Damaged source expression (ship entries only).
    #[serde(default)]
    pub damaged_source: Option<String>,
    /// Slotitem MST ID source expressions (slotitem entries only).
    #[serde(default)]
    pub slot_mst_id_sources: Option<Vec<String>>,
    /// Texture provider name (texture-provider entries only).
    #[serde(default)]
    pub provider: Option<String>,
    /// Texture IDs (texture-provider entries only).
    #[serde(default)]
    pub texture_ids: Option<Vec<i64>>,
    /// Explicit resource paths (explicit-path entries only).
    #[serde(default)]
    pub paths: Option<Vec<String>>,
    /// Module IDs that contain this pattern.
    #[serde(default)]
    pub module_ids: Vec<String>,
    /// Module names that contain this pattern.
    #[serde(default)]
    pub module_names: Vec<String>,
    /// Catch-all for unknown fields.
    #[serde(flatten)]
    pub other: std::collections::HashMap<String, serde_json::Value>,
}

/// Entry kind discriminator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ManifestEntryKind {
    Ship,
    Slotitem,
    #[serde(rename = "texture-provider")]
    TextureProvider,
    #[serde(rename = "explicit-path")]
    ExplicitPath,
    #[serde(untagged)]
    Unknown(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_all_kinds() {
        let json = r#"{
			"version": 1,
			"generatedAt": "2026-01-01T00:00:00Z",
			"entries": [
				{
					"kind": "ship",
					"source": "resources.getShip",
					"targetType": "full",
					"shipMstIdSource": "self.shipModel.mstID",
					"damagedSource": "false",
					"moduleIds": ["123"],
					"moduleNames": ["Test"]
				},
				{
					"kind": "slotitem",
					"source": "SlotLoader.add",
					"targetType": "card",
					"slotMstIdSources": ["this._mst_id"],
					"moduleIds": ["456"],
					"moduleNames": ["Test"]
				},
				{
					"kind": "texture-provider",
					"provider": "COMMON_MISC",
					"textureIds": [1, 2, 3],
					"moduleIds": ["789"],
					"moduleNames": ["Test"]
				},
				{
					"kind": "explicit-path",
					"paths": ["resources/test/abc.png", "kcs2/resources/test/def.png"],
					"moduleIds": ["012"],
					"moduleNames": ["Test"]
				},
				{
					"kind": "future-unknown",
					"moduleIds": [],
					"moduleNames": []
				}
			]
		}"#;

        let manifest: ResourceManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.entries.len(), 5);

        assert_eq!(manifest.entries[0].kind, ManifestEntryKind::Ship);
        assert_eq!(manifest.entries[0].ship_mst_id_source.as_deref(), Some("self.shipModel.mstID"));
        assert_eq!(manifest.entries[0].damaged_source.as_deref(), Some("false"));

        assert_eq!(manifest.entries[1].kind, ManifestEntryKind::Slotitem);
        assert_eq!(manifest.entries[1].slot_mst_id_sources.as_deref().unwrap().len(), 1);

        assert_eq!(manifest.entries[2].kind, ManifestEntryKind::TextureProvider);
        assert_eq!(manifest.entries[2].provider.as_deref(), Some("COMMON_MISC"));

        assert_eq!(manifest.entries[3].kind, ManifestEntryKind::ExplicitPath);
        assert_eq!(manifest.entries[3].paths.as_ref().unwrap().len(), 2);

        assert_eq!(
            manifest.entries[4].kind,
            ManifestEntryKind::Unknown("future-unknown".to_string())
        );
    }

    #[test]
    fn test_forward_compatible_new_fields() {
        let json = r#"{
			"version": 1,
			"generatedAt": "2026-01-01",
			"entries": [{
				"kind": "ship",
				"source": "test",
				"targetType": "full",
				"newField": "value"
			}]
		}"#;

        let manifest: ResourceManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.entries[0].other.get("newField").unwrap(), "value");
    }
}
