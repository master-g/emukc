use std::{collections::HashSet, sync::OnceLock};

pub(crate) mod generate;
mod loader;
pub(crate) mod resolve;
mod types;

pub(crate) use loader::{
    load_cache_rules_bundle, load_cache_rules_bundle_from_path, load_decoder_coverage_assets,
    load_decoder_coverage_assets_from_manifest_path, load_resource_manifest,
    load_resource_manifest_from_path,
};
pub(crate) use types::{
    CacheRuleShipVoiceFormula, CacheRuleShipVoiceRule, CacheRuleSoundBucketRule,
    CacheRuleSoundRules, DecoderCoverageAssets, DecoderRulesBundle, PathRules,
    ResourceCategoriesAsset, ResourceCoverageMode, ResourceManifest, ResourceTemplateFamily,
    ResourceTemplateInput, ResourceTemplatePlaceholderFormat, ResourceTemplateSegmentKind,
    ShipPathHoles,
};

#[cfg(test)]
pub(crate) use types::{
    ResourceTemplateDomain, ResourceTemplateProvenance, ResourceTemplateRange,
    ResourceTemplateSegment, ResourceTemplatesAsset,
};

pub(crate) static PATH_RULES: OnceLock<PathRules> = OnceLock::new();
pub(crate) static BTXT_FLAT_COVERAGE: OnceLock<HashSet<i64>> = OnceLock::new();

pub(crate) fn path_rules() -> Option<&'static PathRules> {
    PATH_RULES.get()
}

pub(crate) fn btxt_flat_coverage() -> Option<&'static HashSet<i64>> {
    BTXT_FLAT_COVERAGE.get()
}

pub(crate) fn populate_path_rules_locks(manifest: &ResourceManifest) {
    populate_path_rules_locks_inner(manifest.path_rules.as_ref(), &PATH_RULES, &BTXT_FLAT_COVERAGE);
}

fn populate_path_rules_locks_inner(
    rules: Option<&PathRules>,
    rules_lock: &OnceLock<PathRules>,
    coverage_lock: &OnceLock<HashSet<i64>>,
) {
    let Some(rules) = rules else {
        return;
    };

    let _ = rules_lock.set(rules.clone());

    if !rules.btxt_flat_slot_ids.is_empty() {
        let _ = coverage_lock.set(rules.btxt_flat_slot_ids.iter().copied().collect());
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::OnceLock};

    use super::*;

    fn make_rules() -> PathRules {
        PathRules {
            btxt_flat_slot_ids: vec![1, 2, 3],
            special_ships: vec![639],
            ..Default::default()
        }
    }

    #[test]
    fn test_populate_path_rules_locks_with_v2_manifest() {
        let manifest = ResourceManifest {
            version: 2,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            summary: Default::default(),
            path_rules: Some(make_rules()),
            entries: Vec::new(),
        };
        let rules_lock = OnceLock::new();
        let coverage_lock: OnceLock<HashSet<i64>> = OnceLock::new();

        populate_path_rules_locks_inner(manifest.path_rules.as_ref(), &rules_lock, &coverage_lock);

        let rules = rules_lock.get().expect("rules lock should be populated");
        assert_eq!(rules.special_ships, vec![639]);
        assert!(coverage_lock.get().unwrap().contains(&1));
        assert!(coverage_lock.get().unwrap().contains(&3));
    }

    #[test]
    fn test_populate_path_rules_locks_with_v1_manifest_keeps_locks_empty() {
        let manifest = ResourceManifest {
            version: 1,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            summary: Default::default(),
            path_rules: None,
            entries: Vec::new(),
        };
        let rules_lock = OnceLock::new();
        let coverage_lock: OnceLock<HashSet<i64>> = OnceLock::new();

        populate_path_rules_locks_inner(manifest.path_rules.as_ref(), &rules_lock, &coverage_lock);

        assert!(rules_lock.get().is_none());
        assert!(coverage_lock.get().is_none());
    }

    #[test]
    fn test_populate_path_rules_locks_skips_empty_btxt_flat_ids() {
        let manifest = ResourceManifest {
            version: 2,
            generated_at: "2026-01-01T00:00:00Z".to_string(),
            summary: Default::default(),
            path_rules: Some(PathRules::default()),
            entries: Vec::new(),
        };
        let rules_lock = OnceLock::new();
        let coverage_lock: OnceLock<HashSet<i64>> = OnceLock::new();

        populate_path_rules_locks_inner(manifest.path_rules.as_ref(), &rules_lock, &coverage_lock);

        assert!(rules_lock.get().is_some());
        assert!(coverage_lock.get().is_none());
    }
}
