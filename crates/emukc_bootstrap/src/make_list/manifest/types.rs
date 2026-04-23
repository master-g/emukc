use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The expected manifest version.
pub(crate) const MANIFEST_VERSION: i64 = 2;

/// Ship hole lists grouped by category.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShipPathHoles {
    #[serde(default)]
    pub full: Vec<i64>,
    #[serde(default)]
    pub full_dmg: Vec<i64>,
    #[serde(default)]
    pub up: Vec<i64>,
    #[serde(default)]
    pub up_dmg: Vec<i64>,
}

/// Path rule data for replacing hardcoded make-list constants.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PathRules {
    /// Mapping from base ship target types to their damage variants.
    #[serde(default)]
    pub ship_damage_variants: BTreeMap<String, Vec<String>>,
    /// Ship target types that use the standard category pattern.
    #[serde(default)]
    pub ship_standard_categories: Vec<String>,
    /// Ship target types that use the `full`/`full_dmg` filename pattern.
    #[serde(default)]
    pub ship_full_categories: Vec<String>,
    /// Slot target types that use the standard category pattern.
    #[serde(default)]
    pub slot_standard_categories: Vec<String>,
    /// Explicit enemy plane IDs used by the default strategy.
    #[serde(default)]
    pub enemy_plane_ids: Vec<i64>,
    /// Explicit `btxt_flat` slotitem IDs.
    #[serde(default)]
    pub btxt_flat_slot_ids: Vec<i64>,
    /// Slotitem IDs that should be excluded from `item_character`.
    #[serde(default)]
    pub character_hole_ids: Vec<i64>,
    /// Event ship hole data.
    #[serde(default)]
    pub event_ship_holes: ShipPathHoles,
    /// Enemy ship hole data.
    #[serde(default)]
    pub enemy_ship_holes: ShipPathHoles,
    /// Ship IDs that have `special` art.
    #[serde(default)]
    pub special_ships: Vec<i64>,
    /// Ship IDs that have SP remodel resources.
    #[serde(default)]
    pub sp_remodel_ships: Vec<i64>,
    /// Ship IDs that have SP remodel message art.
    #[serde(default)]
    pub sp_remodel_mes: Vec<i64>,
    /// Ship IDs that have `card_round`/`icon_box`.
    #[serde(default)]
    pub card_rounds: Vec<i64>,
    /// Ship IDs that have reward resources.
    #[serde(default)]
    pub reward_ships: Vec<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceCategoryEntry {
    pub source: String,
    pub target_type: String,
    #[serde(default)]
    pub module_ids: Vec<String>,
    #[serde(default)]
    pub module_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShipGenerationGroups {
    #[serde(default)]
    pub default_friendly: Vec<String>,
    #[serde(default)]
    pub default_abyssal: Vec<String>,
    #[serde(default)]
    pub friend_graph: Vec<String>,
    #[serde(default)]
    pub enemy_graph: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SlotGenerationGroups {
    #[serde(default)]
    pub default: Vec<String>,
    #[serde(default)]
    pub baga: Vec<String>,
    #[serde(default)]
    pub airunit: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceCategoriesAsset {
    pub version: i64,
    pub generated_at: String,
    pub script_version: String,
    #[serde(default)]
    pub ship_target_types: Vec<ResourceCategoryEntry>,
    #[serde(default)]
    pub slot_target_types: Vec<ResourceCategoryEntry>,
    #[serde(default)]
    pub ship_generation_groups: ShipGenerationGroups,
    #[serde(default)]
    pub slot_generation_groups: SlotGenerationGroups,
    #[serde(default)]
    pub sp_remodel_subcategories: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ResourceCoverageMode {
    #[default]
    Unresolved,
    Partial,
    #[serde(rename = "observed-complete")]
    ObservedComplete,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceIdSetEntry {
    #[serde(default)]
    pub coverage_mode: ResourceCoverageMode,
    #[serde(default)]
    pub ids: Vec<i64>,
    #[serde(default)]
    pub module_ids: Vec<String>,
    #[serde(default)]
    pub module_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShipIdSetsAsset {
    #[serde(default)]
    pub special_ships: ResourceIdSetEntry,
    #[serde(default)]
    pub sp_remodel_ships: ResourceIdSetEntry,
    #[serde(default)]
    pub sp_remodel_message_ships: ResourceIdSetEntry,
    #[serde(default)]
    pub card_round_ships: ResourceIdSetEntry,
    #[serde(default)]
    pub reward_ships: ResourceIdSetEntry,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SlotitemIdSetsAsset {
    #[serde(default)]
    pub btxt_flat_ids: ResourceIdSetEntry,
    #[serde(default)]
    pub item_up_ids: ResourceIdSetEntry,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResourceIdSetsAsset {
    pub version: i64,
    pub generated_at: String,
    pub script_version: String,
    #[serde(default)]
    pub ship_id_sets: ShipIdSetsAsset,
    #[serde(default)]
    pub slotitem_id_sets: SlotitemIdSetsAsset,
    #[serde(default)]
    pub unresolved_keys: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioResourceIdGroup {
    #[serde(default)]
    pub coverage_mode: ResourceCoverageMode,
    #[serde(default)]
    pub ids: Vec<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioVoiceAsset {
    #[serde(default)]
    pub titlecall_categories: Vec<String>,
    #[serde(default)]
    pub tutorial_voice_stems: Vec<String>,
    #[serde(default)]
    pub explicit_files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioBgmAsset {
    #[serde(default)]
    pub fanfare_ids: AudioResourceIdGroup,
    #[serde(default)]
    pub port_ids: AudioResourceIdGroup,
    #[serde(default)]
    pub battle_ids: AudioResourceIdGroup,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AudioResourcesAsset {
    pub version: i64,
    pub generated_at: String,
    pub script_version: String,
    #[serde(default)]
    pub se_ids: AudioResourceIdGroup,
    #[serde(default)]
    pub bgm: AudioBgmAsset,
    #[serde(default)]
    pub voice: AudioVoiceAsset,
    #[serde(default)]
    pub explicit_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiResourcePathGroup {
    #[serde(default)]
    pub coverage_mode: ResourceCoverageMode,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiResourceIdGroup {
    #[serde(default)]
    pub coverage_mode: ResourceCoverageMode,
    #[serde(default)]
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiMapAsset {
    #[serde(default)]
    pub default_files: UiResourcePathGroup,
    #[serde(default)]
    pub event_files: UiResourcePathGroup,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiFurnitureAsset {
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub explicit_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiUseItemAsset {
    #[serde(default)]
    pub card_ids: UiResourceIdGroup,
    #[serde(default)]
    pub underline_ids: UiResourceIdGroup,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiAreaAsset {
    #[serde(default)]
    pub sally_ids: UiResourceIdGroup,
    #[serde(default)]
    pub airunit_ids: UiResourceIdGroup,
    #[serde(default)]
    pub airunit_extend_confirm_ids: UiResourceIdGroup,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiWorldSelectAsset {
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UiResourcesAsset {
    pub version: i64,
    pub generated_at: String,
    pub script_version: String,
    #[serde(default)]
    pub map: UiMapAsset,
    #[serde(default)]
    pub furniture: UiFurnitureAsset,
    #[serde(default)]
    pub use_item: UiUseItemAsset,
    #[serde(default)]
    pub area: UiAreaAsset,
    #[serde(default)]
    pub world_select: UiWorldSelectAsset,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRuleSummary {
    #[serde(default)]
    pub ship_rule_count: i64,
    #[serde(default)]
    pub slot_rule_count: i64,
    #[serde(default)]
    pub observed_complete_rule_count: i64,
    #[serde(default)]
    pub partial_rule_count: i64,
    #[serde(default)]
    pub unresolved_rule_count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRuleSpecialCase {
    #[serde(default)]
    pub damaged: bool,
    #[serde(default)]
    pub ship_ids: Vec<i64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRuleSpecialShipRule {
    #[serde(default)]
    pub coverage_mode: ResourceCoverageMode,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub cases: Vec<CacheRuleSpecialCase>,
    #[serde(default)]
    pub module_ids: Vec<String>,
    #[serde(default)]
    pub module_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRuleExcludeEntry {
    #[serde(rename = "type", default)]
    pub type_: String,
    #[serde(default)]
    pub mst_id: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRuleItemUpRule {
    #[serde(default)]
    pub coverage_mode: ResourceCoverageMode,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub replace_map: BTreeMap<String, i64>,
    #[serde(default)]
    pub enemy_slot_border: Option<i64>,
    #[serde(default)]
    pub exclude: Vec<CacheRuleExcludeEntry>,
    #[serde(default)]
    pub module_ids: Vec<String>,
    #[serde(default)]
    pub module_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRuleBtxtFlatRule {
    #[serde(default)]
    pub coverage_mode: ResourceCoverageMode,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub exclude_enemy_items: bool,
    #[serde(default)]
    pub module_ids: Vec<String>,
    #[serde(default)]
    pub module_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRuleShipRules {
    #[serde(default)]
    pub special: CacheRuleSpecialShipRule,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRuleSlotRules {
    #[serde(default)]
    pub item_up: CacheRuleItemUpRule,
    #[serde(default)]
    pub btxt_flat: CacheRuleBtxtFlatRule,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DecoderCoverageAssets {
    pub resource_categories: Option<ResourceCategoriesAsset>,
    pub resource_id_sets: Option<ResourceIdSetsAsset>,
    pub audio_resources: Option<AudioResourcesAsset>,
    pub ui_resources: Option<UiResourcesAsset>,
}

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
    /// Optional path rules for default/greedy cache list generation.
    #[serde(default)]
    pub path_rules: Option<PathRules>,
    /// Resource entries.
    pub entries: Vec<ResourceManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CacheRulesAsset {
    pub version: i64,
    pub generated_at: String,
    pub script_version: String,
    #[serde(default)]
    pub summary: CacheRuleSummary,
    pub resource_manifest: ResourceManifest,
    #[serde(default)]
    pub resource_categories: ResourceCategoriesAsset,
    #[serde(default)]
    pub ship_rules: CacheRuleShipRules,
    #[serde(default)]
    pub slot_rules: CacheRuleSlotRules,
    #[serde(default)]
    pub unresolved_rules: Vec<String>,
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
        assert!(manifest.path_rules.is_none());
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

    #[test]
    fn test_path_rules_deserialize_v2() {
        let json = r#"{
			"version": 2,
			"generatedAt": "2026-01-01T00:00:00Z",
			"pathRules": {
				"shipDamageVariants": {
					"banner": ["banner_dmg", "banner_g_dmg", "banner_g"]
				},
				"shipStandardCategories": ["banner", "banner_dmg"],
				"shipFullCategories": ["full", "full_dmg"],
				"slotStandardCategories": ["card", "item_on"],
				"enemyPlaneIds": [1, 2, 3],
				"btxtFlatSlotIds": [1, 2, 3],
				"characterHoleIds": [42],
				"eventShipHoles": {
					"full": [5001],
					"fullDmg": [5002],
					"up": [5003],
					"upDmg": [5004]
				},
				"enemyShipHoles": {
					"full": [1501]
				},
				"specialShips": [639],
				"spRemodelShips": [501],
				"spRemodelMes": [73],
				"cardRounds": [524],
				"rewardShips": [900]
			},
			"entries": []
		}"#;

        let manifest: ResourceManifest = serde_json::from_str(json).unwrap();
        let rules = manifest.path_rules.expect("pathRules should deserialize");
        assert_eq!(manifest.version, 2);
        assert_eq!(
            rules.ship_damage_variants.get("banner").unwrap(),
            &vec!["banner_dmg".to_string(), "banner_g_dmg".to_string(), "banner_g".to_string()]
        );
        assert_eq!(rules.enemy_plane_ids, vec![1, 2, 3]);
        assert_eq!(rules.event_ship_holes.full, vec![5001]);
        assert_eq!(rules.enemy_ship_holes.full, vec![1501]);
    }

    #[test]
    fn test_path_rules_partial_fields_default_to_empty() {
        let json = r#"{
			"version": 2,
			"generatedAt": "2026-01-01T00:00:00Z",
			"pathRules": {
				"specialShips": [639]
			},
			"entries": []
		}"#;

        let manifest: ResourceManifest = serde_json::from_str(json).unwrap();
        let rules = manifest.path_rules.expect("pathRules should deserialize");
        assert_eq!(rules.special_ships, vec![639]);
        assert!(rules.ship_damage_variants.is_empty());
        assert!(rules.btxt_flat_slot_ids.is_empty());
        assert_eq!(rules.event_ship_holes, ShipPathHoles::default());
        assert_eq!(rules.enemy_ship_holes, ShipPathHoles::default());
    }
}
