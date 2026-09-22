use std::{
    collections::HashSet,
    sync::{LazyLock, OnceLock},
};

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
    CacheRuleItemUpRule, CacheRuleShipVoiceFormula, CacheRuleShipVoiceRule,
    CacheRuleSoundBucketRule, CacheRuleSoundRules, DecoderCoverageAssets, DecoderRulesBundle,
    PathRules, ResourceCategoriesAsset, ResourceCoverageMode, ResourceManifest,
    ResourceTemplateFamily, ResourceTemplateInput, ResourceTemplatePlaceholderFormat,
    ResourceTemplateSegmentKind, ShipPathHoles,
};

#[cfg(test)]
pub(crate) use types::{
    ResourceTemplateDomain, ResourceTemplateProvenance, ResourceTemplateRange,
    ResourceTemplateSegment, ResourceTemplatesAsset, SlotGenerationGroups,
};

pub(crate) static PATH_RULES: OnceLock<PathRules> = OnceLock::new();
pub(crate) static BTXT_FLAT_COVERAGE: OnceLock<HashSet<i64>> = OnceLock::new();

pub(crate) fn path_rules() -> Option<&'static PathRules> {
    PATH_RULES.get()
}

pub(crate) fn btxt_flat_coverage() -> Option<&'static HashSet<i64>> {
    BTXT_FLAT_COVERAGE.get()
}

/// The slot id `item_up` generation actually emits a path for. Abyssal ids are
/// either remapped outright (`replaceMap`) or folded below the enemy-slot
/// border, so a raw `api_eSlot` id and the generated path disagree without this.
pub(crate) fn normalize_item_up_slot_id(rule: &CacheRuleItemUpRule, slot_id: i64) -> i64 {
    if let Some(replaced) = rule.replace_map.get(&slot_id.to_string()) {
        *replaced
    } else if let Some(border) = rule.enemy_slot_border.filter(|border| slot_id > *border) {
        slot_id - border
    } else {
        slot_id
    }
}

/// The repo's `item_up` rule, for callers that have no loaded rules bundle of
/// their own. Reads the synced asset off disk once; `None` when it is missing
/// or still unresolved, in which case ids pass through unchanged.
static REPO_ITEM_UP_RULE: LazyLock<Option<CacheRuleItemUpRule>> = LazyLock::new(|| {
    let bundle = load_cache_rules_bundle().ok()?;
    let rule = bundle.cache_rules.slot_rules.item_up;
    (rule.coverage_mode != ResourceCoverageMode::Unresolved).then_some(rule)
});

pub(crate) fn repo_item_up_slot_id(slot_id: i64) -> i64 {
    REPO_ITEM_UP_RULE.as_ref().map_or(slot_id, |rule| normalize_item_up_slot_id(rule, slot_id))
}

/// Whether `item_up` generation emits a path for this (already normalized) id.
pub(crate) fn has_repo_item_up_coverage(slot_id: i64) -> bool {
    REPO_ITEM_UP_RULE.as_ref().is_none_or(|rule| {
        !rule.exclude.iter().any(|entry| entry.type_ == "item_up" && entry.mst_id == slot_id)
    })
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
            script_version: None,
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
            script_version: None,
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
            script_version: None,
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
