use emukc_cache::IntoVersion;
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::{ApiManifest, ApiMstSlotitem};

use super::resolve;
use super::types::{
    CacheRuleDamagedState, CacheRuleShipSelectorScope, CacheRulesAsset, DecoderCoverageAssets,
    ManifestEntryKind, PathRules, ResourceCoverageMode, ResourceIdSetEntry, ResourceManifestEntry,
};
use crate::make_list::CacheList;

/// Mapping from base ship target types to their damage variant target types.
const SHIP_DAMAGE_VARIANTS: &[(&str, &[&str])] = &[
    ("banner", &["banner_dmg", "banner_g_dmg", "banner_g"]),
    ("banner2", &["banner2_dmg", "banner2_g_dmg", "banner2_g"]),
    ("banner3", &["banner3_g_dmg", "banner3_g"]),
    ("card", &["card_dmg"]),
    ("full", &["full_dmg"]),
    ("character_full", &["character_full_dmg"]),
    ("character_up", &["character_up_dmg"]),
    ("remodel", &["remodel_dmg"]),
    ("supply_character", &["supply_character_dmg"]),
];

/// Ship target types that use the standard `{category}/{id:04}_{suffix}.png` pattern.
const SHIP_STANDARD_CATEGORIES: &[&str] = &[
    "album_status",
    "banner",
    "banner2",
    "banner2_dmg",
    "banner2_g",
    "banner2_g_dmg",
    "banner3",
    "banner3_g",
    "banner3_g_dmg",
    "banner_dmg",
    "banner_g",
    "banner_g_dmg",
    "card",
    "card_dmg",
    "card_round",
    "character_full",
    "character_full_dmg",
    "character_up",
    "character_up_dmg",
    "icon_box",
    "power_up",
    "remodel",
    "remodel_dmg",
    "reward_card",
    "reward_icon",
    "reward_icon",
    "special",
    "supply_character",
    "supply_character_dmg",
];

/// Ship target types that use `full`/`full_dmg` pattern (with `api_filename`).
const SHIP_FULL_CATEGORIES: &[&str] = &["full", "full_dmg"];

/// SP remodel sub-categories.
const _SP_REMODEL_CATEGORIES: &[&str] = &[
    "sp_remodel/full_x2",
    "sp_remodel/silhouette",
    "sp_remodel/text_class",
    "sp_remodel/text_name",
    "sp_remodel/text_remodel_mes",
];

/// Slotitem target types.
const SLOT_STANDARD_CATEGORIES: &[&str] = &[
    "card",
    "card_t",
    "item_on",
    "item_on2",
    "item_up",
    "item_up2",
    "remodel",
    "statustop_item",
    "airunit_banner",
    "airunit_fairy",
    "airunit_name",
    "btxt_flat",
    "item_character",
];

/// Look up damage variant target types for a base type. Returns empty slice if none.
fn get_damage_variants(base_type: &str) -> &[&str] {
    SHIP_DAMAGE_VARIANTS
        .iter()
        .find(|(base, _)| *base == base_type)
        .map(|(_, variants)| *variants)
        .unwrap_or(&[])
}

fn should_use_observed_ids(entry: &ResourceIdSetEntry) -> bool {
    entry.coverage_mode == ResourceCoverageMode::ObservedComplete && !entry.ids.is_empty()
}

fn pick_longest_id_set(candidates: [Option<Vec<i64>>; 3]) -> Option<Vec<i64>> {
    let mut best: Option<Vec<i64>> = None;
    for ids in candidates.into_iter().flatten() {
        if best.as_ref().is_none_or(|current| ids.len() > current.len()) {
            best = Some(ids);
        }
    }
    best
}

fn sparse_ship_ids_from_rules(target: &str, path_rules: Option<&PathRules>) -> Option<Vec<i64>> {
    let rules = path_rules?;
    let ids = match target {
        "special" => &rules.special_ships,
        "card_round" | "icon_box" => &rules.card_rounds,
        "reward_card" | "reward_icon" => &rules.reward_ships,
        "sp_remodel/text_remodel_mes" => &rules.sp_remodel_mes,
        t if t.starts_with("sp_remodel/") => &rules.sp_remodel_ships,
        _ => return None,
    };
    (!ids.is_empty()).then(|| ids.clone())
}

fn sparse_ship_ids_from_decoder(
    target: &str,
    decoder_assets: Option<&DecoderCoverageAssets>,
) -> Option<Vec<i64>> {
    let id_sets = decoder_assets.and_then(|assets| assets.resource_id_sets.as_ref())?;
    let entry = match target {
        "special" => &id_sets.ship_id_sets.special_ships,
        "card_round" | "icon_box" => &id_sets.ship_id_sets.card_round_ships,
        "reward_card" | "reward_icon" => &id_sets.ship_id_sets.reward_ships,
        "sp_remodel/text_remodel_mes" => &id_sets.ship_id_sets.sp_remodel_message_ships,
        t if t.starts_with("sp_remodel/") => &id_sets.ship_id_sets.sp_remodel_ships,
        _ => return None,
    };
    should_use_observed_ids(entry).then(|| entry.ids.clone())
}

fn sparse_ship_ids_from_cache_rules(
    target: &str,
    cache_rules: Option<&CacheRulesAsset>,
) -> Option<Vec<i64>> {
    if target != "special" {
        return None;
    }

    let rule = &cache_rules?.ship_rules.special;
    if rule.coverage_mode == ResourceCoverageMode::Unresolved {
        return None;
    }

    let mut ids = rule
        .cases
        .iter()
        .filter(|case| !case.damaged)
        .flat_map(|case| case.ship_ids.iter().copied())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    (!ids.is_empty()).then_some(ids)
}

fn default_group_ship_ids_from_categories(
    target: &str,
    mst: &ApiManifest,
    categories: &super::types::ResourceCategoriesAsset,
) -> Option<Vec<i64>> {
    let groups = &categories.ship_generation_groups;
    let mut ids = Vec::new();

    if groups.default_friendly.iter().any(|category| category == target) {
        ids.extend(
            mst.api_mst_ship
                .iter()
                .filter(|ship| ship.api_aftershipid.is_some())
                .map(|ship| ship.api_id),
        );
    }

    if groups.default_abyssal.iter().any(|category| category == target) {
        ids.extend(
            mst.api_mst_ship
                .iter()
                .filter(|ship| ship.api_aftershipid.is_none())
                .map(|ship| ship.api_id),
        );
    }

    ids.sort_unstable();
    ids.dedup();
    (!ids.is_empty()).then_some(ids)
}

fn default_group_ship_ids_from_cache_rules(
    target: &str,
    mst: &ApiManifest,
    cache_rules: Option<&CacheRulesAsset>,
) -> Option<Vec<i64>> {
    default_group_ship_ids_from_categories(target, mst, &cache_rules?.resource_categories)
}

fn graph_group_ship_ids_from_cache_rules(
    target: &str,
    mst: &ApiManifest,
    cache_rules: Option<&CacheRulesAsset>,
) -> Option<Vec<i64>> {
    let groups = &cache_rules?.resource_categories.ship_generation_groups;
    let mut ids = Vec::new();

    let uses_friend_graph = groups.friend_graph.iter().any(|category| category == target);
    let uses_enemy_graph = groups.enemy_graph.iter().any(|category| category == target);

    if uses_friend_graph {
        ids.extend(
            mst.api_mst_shipgraph
                .iter()
                .filter(|graph| {
                    graph.api_sortno.is_some_and(|s| s > 0) && !graph.api_version.is_empty()
                })
                .map(|graph| graph.api_id),
        );

        if matches!(
            target,
            "character_full" | "character_full_dmg" | "character_up" | "character_up_dmg"
        ) {
            ids.extend(
                mst.api_mst_shipgraph
                    .iter()
                    .filter(|graph| graph.api_id > 5000)
                    .map(|graph| graph.api_id),
            );
        }
    }

    if uses_enemy_graph {
        ids.extend(
            mst.api_mst_shipgraph
                .iter()
                .filter(|graph| graph.api_sortno.is_none() && graph.api_id < 5000)
                .map(|graph| graph.api_id),
        );
    }

    ids.sort_unstable();
    ids.dedup();
    (!ids.is_empty()).then_some(ids)
}

fn damaged_state_from_option(damaged: Option<bool>) -> CacheRuleDamagedState {
    match damaged {
        Some(false) => CacheRuleDamagedState::False,
        Some(true) => CacheRuleDamagedState::True,
        None => CacheRuleDamagedState::Variable,
    }
}

fn ship_selector_scope_for_id(
    ship_id: i64,
    mst: &ApiManifest,
) -> Option<CacheRuleShipSelectorScope> {
    let ship = mst.api_mst_ship.iter().find(|ship| ship.api_id == ship_id)?;
    if ship.api_aftershipid.is_some() {
        Some(CacheRuleShipSelectorScope::DefaultFriendly)
    } else {
        Some(CacheRuleShipSelectorScope::DefaultAbyssal)
    }
}

fn has_ship_target_semantics(target: &str, cache_rules: Option<&CacheRulesAsset>) -> bool {
    let Some(rule) = cache_rules
        .map(|rules| &rules.ship_rules.target_semantics)
        .filter(|rule| rule.coverage_mode != ResourceCoverageMode::Unresolved)
    else {
        return false;
    };

    rule.cases.iter().any(|case| case.raw_target_type == target)
}

fn ship_semantic_targets_for_id(
    target: &str,
    damaged: Option<bool>,
    ship_id: i64,
    mst: &ApiManifest,
    cache_rules: Option<&CacheRulesAsset>,
) -> Option<Vec<String>> {
    let rule = cache_rules
        .map(|rules| &rules.ship_rules.target_semantics)
        .filter(|rule| rule.coverage_mode != ResourceCoverageMode::Unresolved)?;

    let has_cases = rule.cases.iter().any(|case| case.raw_target_type == target);
    if !has_cases {
        return None;
    }

    let scope = ship_selector_scope_for_id(ship_id, mst);
    let damaged_state = damaged_state_from_option(damaged);
    let mut target_types = rule
        .cases
        .iter()
        .filter(|case| {
            case.raw_target_type == target
                && scope.as_ref().is_some_and(|scope| &case.selector_scope == scope)
                && case.damaged_state == damaged_state
        })
        .flat_map(|case| case.target_types.iter().cloned())
        .collect::<Vec<_>>();
    target_types.sort();
    target_types.dedup();
    Some(target_types)
}

fn observed_slot_subset_from_cache_rules(
    target: &str,
    cache_rules: Option<&CacheRulesAsset>,
) -> Option<Vec<i64>> {
    let rule = match target {
        "item_up2" => &cache_rules?.slot_rules.item_up2,
        "item_on2" => &cache_rules?.slot_rules.item_on2,
        _ => return None,
    };

    (rule.coverage_mode == ResourceCoverageMode::ObservedComplete && !rule.ids.is_empty())
        .then(|| rule.ids.clone())
}

fn should_skip_ship_category(
    ship_id: i64,
    category: &str,
    graph: Option<&emukc_model::kc2::start2::ApiMstShipgraph>,
    path_rules: Option<&PathRules>,
) -> bool {
    let Some(graph) = graph else {
        return false;
    };
    let Some(rules) = path_rules else {
        return false;
    };

    if ship_id > 5000 {
        return match category {
            "character_full" => rules.event_ship_holes.full.contains(&ship_id),
            "character_full_dmg" => rules.event_ship_holes.full_dmg.contains(&ship_id),
            "character_up" => rules.event_ship_holes.up.contains(&ship_id),
            "character_up_dmg" => rules.event_ship_holes.up_dmg.contains(&ship_id),
            _ => false,
        };
    }

    if graph.api_sortno.is_none() {
        return match category {
            "full" => rules.enemy_ship_holes.full.contains(&ship_id),
            "full_dmg" => rules.enemy_ship_holes.full_dmg.contains(&ship_id),
            _ => false,
        };
    }

    false
}

fn resolve_ship_ids_for_target(
    source: &str,
    target: &str,
    mst: &ApiManifest,
    path_rules: Option<&PathRules>,
    decoder_assets: Option<&DecoderCoverageAssets>,
    cache_rules: Option<&CacheRulesAsset>,
) -> Vec<i64> {
    pick_longest_id_set([
        sparse_ship_ids_from_cache_rules(target, cache_rules),
        sparse_ship_ids_from_decoder(target, decoder_assets),
        sparse_ship_ids_from_rules(target, path_rules),
    ])
    .or_else(|| default_group_ship_ids_from_cache_rules(target, mst, cache_rules))
    .or_else(|| graph_group_ship_ids_from_cache_rules(target, mst, cache_rules))
    .unwrap_or_else(|| resolve::resolve_ship_ids(source, mst))
}

fn airunit_slot_ids(mst: &ApiManifest) -> Vec<i64> {
    let mut plane_slots: std::collections::BTreeMap<i64, &ApiMstSlotitem> =
        std::collections::BTreeMap::new();
    for slot in mst.api_mst_slotitem.iter() {
        if let Some(key) =
            (slot.api_type[4] != 0 && slot.api_sortno > 0).then_some(slot.api_type[4])
        {
            plane_slots
                .entry(key)
                .and_modify(|entry| {
                    if entry.api_version.is_none() && slot.api_version.is_some() {
                        *entry = slot;
                    }
                })
                .or_insert(slot);
        }
    }
    plane_slots.values().map(|slot| slot.api_id).collect()
}

fn resolve_slot_ids_for_target(
    sources: &[String],
    target: &str,
    mst: &ApiManifest,
    path_rules: Option<&PathRules>,
    decoder_assets: Option<&DecoderCoverageAssets>,
    cache_rules: Option<&CacheRulesAsset>,
) -> Vec<i64> {
    if let Some(ids) = observed_slot_subset_from_cache_rules(target, cache_rules) {
        return ids;
    }

    if target == "item_up" {
        if let Some(rule) = cache_rules
            .map(|rules| &rules.slot_rules.item_up)
            .filter(|rule| rule.coverage_mode != ResourceCoverageMode::Unresolved)
        {
            let exclude = rule
                .exclude
                .iter()
                .filter(|entry| entry.type_ == "item_up")
                .map(|entry| entry.mst_id)
                .collect::<std::collections::BTreeSet<_>>();
            let mut ids = resolve::resolve_slotitem_ids(sources, mst)
                .into_iter()
                .filter(|slot_id| !exclude.contains(slot_id))
                .map(|slot_id| {
                    if let Some(replaced) = rule.replace_map.get(&slot_id.to_string()) {
                        *replaced
                    } else if let Some(border) = rule.enemy_slot_border {
                        if slot_id > border {
                            slot_id - border
                        } else {
                            slot_id
                        }
                    } else {
                        slot_id
                    }
                })
                .filter(|slot_id| *slot_id > 0)
                .collect::<Vec<_>>();
            ids.sort_unstable();
            ids.dedup();
            if !ids.is_empty() {
                return ids;
            }
        }
    }

    if target == "btxt_flat" {
        if let Some(ids) = pick_longest_id_set([
            None,
            decoder_assets
                .and_then(|assets| assets.resource_id_sets.as_ref())
                .map(|id_sets| &id_sets.slotitem_id_sets.btxt_flat_ids)
                .filter(|entry| should_use_observed_ids(entry))
                .map(|entry| entry.ids.clone()),
            path_rules
                .filter(|rules| !rules.btxt_flat_slot_ids.is_empty())
                .map(|rules| rules.btxt_flat_slot_ids.clone()),
        ]) {
            return ids;
        }

        if let Some(rule) = cache_rules.map(|rules| &rules.slot_rules.btxt_flat).filter(|rule| {
            rule.coverage_mode != ResourceCoverageMode::Unresolved && rule.exclude_enemy_items
        }) {
            let enemy_slot_border = cache_rules
                .and_then(|rules| rules.slot_rules.item_up.enemy_slot_border)
                .unwrap_or(i64::MAX);
            let mut ids = mst
                .api_mst_slotitem
                .iter()
                .filter(|slot| slot.api_sortno > 0 && slot.api_id <= enemy_slot_border)
                .map(|slot| slot.api_id)
                .collect::<Vec<_>>();
            if !ids.is_empty() {
                ids.sort_unstable();
                ids.dedup();
                let _ = rule;
                return ids;
            }
        }
    }

    if matches!(target, "airunit_banner" | "airunit_fairy" | "airunit_name") {
        return airunit_slot_ids(mst);
    }

    resolve::resolve_slotitem_ids(sources, mst)
}

pub(crate) fn generate_entry_paths(
    entry: &ResourceManifestEntry,
    mst: &ApiManifest,
    path_rules: Option<&PathRules>,
    decoder_assets: Option<&DecoderCoverageAssets>,
    cache_rules: Option<&CacheRulesAsset>,
    list: &mut CacheList,
) {
    match entry.kind {
        ManifestEntryKind::Ship => {
            generate_ship_paths(entry, mst, path_rules, decoder_assets, cache_rules, list);
        }
        ManifestEntryKind::Slotitem => {
            generate_slotitem_paths(entry, mst, path_rules, decoder_assets, cache_rules, list);
        }
        ManifestEntryKind::ExplicitPath => generate_explicit_paths(entry, list),
        ManifestEntryKind::TextureProvider => {
            // Deferred to future phase
        }
        ManifestEntryKind::Unknown(ref k) => {
            warn!("Skipping unknown manifest entry kind: {k}");
        }
    }
}

fn generate_ship_paths(
    entry: &ResourceManifestEntry,
    mst: &ApiManifest,
    path_rules: Option<&PathRules>,
    decoder_assets: Option<&DecoderCoverageAssets>,
    cache_rules: Option<&CacheRulesAsset>,
    list: &mut CacheList,
) {
    let Some(ref source) = entry.ship_mst_id_source else {
        return;
    };

    let ship_ids = resolve_ship_ids_for_target(
        source,
        entry.target_type.as_str(),
        mst,
        path_rules,
        decoder_assets,
        cache_rules,
    );
    if ship_ids.is_empty() {
        return;
    }

    let damaged = entry.damaged_source.as_deref().and_then(resolve::resolve_damaged);

    let target = entry.target_type.as_str();
    let variants = path_rules
        .and_then(|rules| rules.ship_damage_variants.get(target))
        .filter(|variants| !variants.is_empty())
        .map(|variants| variants.iter().map(String::as_str).collect::<Vec<_>>())
        .unwrap_or_else(|| get_damage_variants(target).to_vec());

    for id in ship_ids {
        let ship_id = format!("{id:04}");

        // Get version from shipgraph
        let version = mst
            .api_mst_shipgraph
            .iter()
            .find(|g| g.api_id == id)
            .and_then(|g| g.api_version.first().into_version());

        // Check if this is a sp_remodel target
        if target.starts_with("sp_remodel") {
            let suffix = SuffixUtils::create(&ship_id, format!("ship_{target}").as_str());
            list.add(
                format!("kcs2/resources/ship/{target}/{ship_id}_{suffix}.png"),
                version.as_ref(),
            );
            continue;
        }

        if has_ship_target_semantics(target, cache_rules) {
            let semantic_targets =
                ship_semantic_targets_for_id(target, damaged, id, mst, cache_rules)
                    .unwrap_or_default();

            for semantic_target in semantic_targets {
                if should_skip_ship_category(
                    id,
                    semantic_target.as_str(),
                    mst.api_mst_shipgraph.iter().find(|g| g.api_id == id),
                    path_rules,
                ) {
                    continue;
                }

                let suffix =
                    SuffixUtils::create(&ship_id, format!("ship_{semantic_target}").as_str());
                list.add(
                    format!("kcs2/resources/ship/{semantic_target}/{ship_id}_{suffix}.png"),
                    version.as_ref(),
                );
            }
            continue;
        }

        // Check if this is a full/full_dmg category (uses api_filename)
        let uses_full_categories = path_rules
            .map(|rules| {
                if rules.ship_full_categories.is_empty() {
                    SHIP_FULL_CATEGORIES.contains(&target)
                } else {
                    rules.ship_full_categories.iter().any(|category| category == target)
                }
            })
            .unwrap_or_else(|| SHIP_FULL_CATEGORIES.contains(&target));

        if uses_full_categories {
            let graph = mst.api_mst_shipgraph.iter().find(|g| g.api_id == id);
            let Some(graph) = graph else {
                continue;
            };

            let gen_base = !matches!(damaged, Some(true));
            let gen_dmg = !matches!(damaged, Some(false));

            for (cat, should_gen) in [("full", gen_base), ("full_dmg", gen_dmg)] {
                if !should_gen {
                    continue;
                }
                if should_skip_ship_category(id, cat, Some(graph), path_rules) {
                    continue;
                }
                let suffix = SuffixUtils::create(&ship_id, format!("ship_{cat}").as_str());
                list.add(
                    format!(
                        "kcs2/resources/ship/{cat}/{ship_id}_{suffix}_{}.png",
                        graph.api_filename
                    ),
                    version.as_ref(),
                );
            }
            continue;
        }

        // Standard category pattern
        let uses_standard_categories = path_rules
            .map(|rules| {
                if rules.ship_standard_categories.is_empty() {
                    SHIP_STANDARD_CATEGORIES.contains(&target)
                } else {
                    rules.ship_standard_categories.iter().any(|category| category == target)
                }
            })
            .unwrap_or_else(|| SHIP_STANDARD_CATEGORIES.contains(&target));

        if uses_standard_categories {
            let gen_base = !matches!(damaged, Some(true));
            let gen_variants = damaged != Some(false) && !variants.is_empty();

            // Generate base path
            if gen_base
                && !should_skip_ship_category(
                    id,
                    target,
                    mst.api_mst_shipgraph.iter().find(|g| g.api_id == id),
                    path_rules,
                )
            {
                let suffix = SuffixUtils::create(&ship_id, format!("ship_{target}").as_str());
                list.add(
                    format!("kcs2/resources/ship/{target}/{ship_id}_{suffix}.png"),
                    version.as_ref(),
                );
            }

            // Generate damage variant paths
            if gen_variants {
                for variant in &variants {
                    if should_skip_ship_category(
                        id,
                        variant,
                        mst.api_mst_shipgraph.iter().find(|g| g.api_id == id),
                        path_rules,
                    ) {
                        continue;
                    }
                    let suffix = SuffixUtils::create(&ship_id, format!("ship_{variant}").as_str());
                    list.add(
                        format!("kcs2/resources/ship/{variant}/{ship_id}_{suffix}.png"),
                        version.as_ref(),
                    );
                }
            }
        } else {
            warn!("Unknown ship target type: {target}");
        }
    }
}

fn generate_slotitem_paths(
    entry: &ResourceManifestEntry,
    mst: &ApiManifest,
    path_rules: Option<&PathRules>,
    decoder_assets: Option<&DecoderCoverageAssets>,
    cache_rules: Option<&CacheRulesAsset>,
    list: &mut CacheList,
) {
    let sources = entry.slot_mst_id_sources.as_deref().unwrap_or(&[]);
    let slot_ids = resolve_slot_ids_for_target(
        sources,
        entry.target_type.as_str(),
        mst,
        path_rules,
        decoder_assets,
        cache_rules,
    );
    if slot_ids.is_empty() {
        return;
    }

    let target = entry.target_type.as_str();

    for id in slot_ids {
        let item_id = format!("{id:04}");

        let version =
            mst.api_mst_slotitem.iter().find(|s| s.api_id == id).and_then(|s| s.api_version);

        let uses_standard_categories = path_rules
            .map(|rules| {
                if rules.slot_standard_categories.is_empty() {
                    SLOT_STANDARD_CATEGORIES.contains(&target)
                } else {
                    rules.slot_standard_categories.iter().any(|category| category == target)
                }
            })
            .unwrap_or_else(|| SLOT_STANDARD_CATEGORIES.contains(&target));

        if uses_standard_categories {
            let suffix = SuffixUtils::create(&item_id, format!("slot_{target}").as_str());
            list.add(format!("kcs2/resources/slot/{target}/{item_id}_{suffix}.png"), version);
        } else {
            warn!("Unknown slotitem target type: {target}");
        }
    }
}

fn generate_explicit_paths(entry: &ResourceManifestEntry, list: &mut CacheList) {
    let Some(ref paths) = entry.paths else {
        return;
    };

    for path in paths {
        // Paths in manifest may or may not have kcs2/ prefix
        let full_path = if path.starts_with("kcs2/") {
            path.clone()
        } else if path.starts_with("resources/") {
            format!("kcs2/{path}")
        } else {
            path.clone()
        };

        // Skip directory paths (ending with /)
        if full_path.ends_with('/') {
            continue;
        }

        // Skip paths that look like directories but lack trailing /
        // (e.g., "resources/voice", "resources/friendly_panel/e")
        let last_segment = full_path.rsplit('/').next().unwrap_or("");
        if !last_segment.contains('.') {
            continue;
        }

        // Skip JSON files that aren't resources
        if full_path.ends_with(".json") && !full_path.contains("ship_image") {
            continue;
        }

        list.add_unversioned(full_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::make_list::manifest::load_resource_manifest;
    use crate::make_list::manifest::types::{
        CacheRuleBtxtFlatRule, CacheRuleDamagedState, CacheRuleExcludeEntry, CacheRuleItemUpRule,
        CacheRuleObservedSlotSubsetRule, CacheRuleShipRules, CacheRuleShipSelectorScope,
        CacheRuleShipTargetSemanticCase, CacheRuleShipTargetSemanticsRule, CacheRuleSlotRules,
        CacheRuleSpecialCase, CacheRuleSpecialShipRule, CacheRulesAsset, PathRules,
        ResourceCategoriesAsset, ResourceManifest, ShipGenerationGroups, ShipPathHoles,
    };
    use emukc_model::kc2::start2::{ApiMstShip, ApiMstShipgraph, ApiMstSlotitem};

    fn make_minimal_manifest() -> ApiManifest {
        ApiManifest {
            api_mst_ship: vec![
                ApiMstShip {
                    api_id: 1,
                    api_sortno: Some(1),
                    api_aftershipid: Some("2".to_string()),
                    api_name: "TestShip".to_string(),
                    ..Default::default()
                },
                ApiMstShip {
                    api_id: 1500,
                    api_sortno: None,
                    api_aftershipid: None,
                    api_name: "Abyssal".to_string(),
                    ..Default::default()
                },
            ],
            api_mst_shipgraph: vec![
                ApiMstShipgraph {
                    api_id: 1,
                    api_sortno: Some(1),
                    api_filename: "1".to_string(),
                    api_version: vec!["1".to_string()],
                    ..Default::default()
                },
                ApiMstShipgraph {
                    api_id: 1500,
                    api_sortno: None,
                    api_filename: "1500".to_string(),
                    api_version: vec![],
                    ..Default::default()
                },
            ],
            api_mst_slotitem: vec![ApiMstSlotitem {
                api_id: 1,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn make_ship_entry(target: &str, source: &str, damaged: Option<&str>) -> ResourceManifestEntry {
        ResourceManifestEntry {
            kind: ManifestEntryKind::Ship,
            source: "test".to_string(),
            target_type: target.to_string(),
            ship_mst_id_source: Some(source.to_string()),
            damaged_source: damaged.map(std::string::ToString::to_string),
            slot_mst_id_sources: None,
            provider: None,
            texture_ids: None,
            paths: None,
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        }
    }

    #[test]
    fn test_generate_ship_banner_paths() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = make_ship_entry("banner", "this._mst_id", Some("false"));
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        assert!(!list.items.is_empty());
        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("ship/banner/")));
    }

    #[test]
    fn test_generate_slotitem_card_paths() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = ResourceManifestEntry {
            kind: ManifestEntryKind::Slotitem,
            source: "test".to_string(),
            target_type: "card".to_string(),
            ship_mst_id_source: None,
            damaged_source: None,
            slot_mst_id_sources: Some(vec!["this._mst_id".to_string()]),
            provider: None,
            texture_ids: None,
            paths: None,
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        };
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        assert!(!list.items.is_empty());
        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("slot/card/")));
    }

    #[test]
    fn test_real_manifest_path_rules_match_generate_constants() {
        let rules = load_resource_manifest()
            .unwrap()
            .path_rules
            .expect("real manifest should include pathRules");

        let expected_damage_variants = SHIP_DAMAGE_VARIANTS
            .iter()
            .map(|(base, variants)| {
                (
                    (*base).to_string(),
                    variants.iter().map(|variant| (*variant).to_string()).collect::<Vec<_>>(),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();

        assert_eq!(rules.ship_damage_variants, expected_damage_variants);
        assert_eq!(
            rules.ship_standard_categories,
            SHIP_STANDARD_CATEGORIES
                .iter()
                .map(|category| (*category).to_string())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            rules.ship_full_categories,
            SHIP_FULL_CATEGORIES.iter().map(|category| (*category).to_string()).collect::<Vec<_>>()
        );
        assert_eq!(
            rules.slot_standard_categories,
            SLOT_STANDARD_CATEGORIES
                .iter()
                .map(|category| (*category).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_generate_entry_paths_with_and_without_path_rules_match() {
        let mst = make_minimal_manifest();
        let rules = load_resource_manifest()
            .unwrap()
            .path_rules
            .expect("real manifest should include pathRules");
        let ship_entry = make_ship_entry("banner", "this._mst_id", Some("_0x1a3f79"));
        let slot_entry = ResourceManifestEntry {
            kind: ManifestEntryKind::Slotitem,
            source: "test".to_string(),
            target_type: "card".to_string(),
            ship_mst_id_source: None,
            damaged_source: None,
            slot_mst_id_sources: Some(vec!["this._mst_id".to_string()]),
            provider: None,
            texture_ids: None,
            paths: None,
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        };

        let mut fallback_list = CacheList::new();
        generate_entry_paths(&ship_entry, &mst, None, None, None, &mut fallback_list);
        generate_entry_paths(&slot_entry, &mst, None, None, None, &mut fallback_list);

        let mut rule_list = CacheList::new();
        generate_entry_paths(&ship_entry, &mst, Some(&rules), None, None, &mut rule_list);
        generate_entry_paths(&slot_entry, &mst, Some(&rules), None, None, &mut rule_list);

        let fallback_paths =
            fallback_list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        let rule_paths = rule_list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(rule_paths, fallback_paths);
    }

    #[test]
    fn test_generate_explicit_paths_skips_directories() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = ResourceManifestEntry {
            kind: ManifestEntryKind::ExplicitPath,
            source: String::new(),
            target_type: String::new(),
            ship_mst_id_source: None,
            damaged_source: None,
            slot_mst_id_sources: None,
            provider: None,
            texture_ids: None,
            paths: Some(vec![
                "resources/ship/".to_string(),            // directory with / — skip
                "resources/voice".to_string(),            // directory without / — skip
                "resources/friendly_panel/e".to_string(), // directory without / — skip
                "resources/stype/etext/sp001.png".to_string(), // file — include
            ]),
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        };
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        assert_eq!(list.items.len(), 1);
        assert!(list.items.iter().any(|i| i.path.contains("sp001.png")));
    }

    #[test]
    fn test_get_damage_variants_banner() {
        assert_eq!(get_damage_variants("banner"), ["banner_dmg", "banner_g_dmg", "banner_g"]);
    }

    #[test]
    fn test_get_damage_variants_card() {
        assert_eq!(get_damage_variants("card"), ["card_dmg"]);
    }

    #[test]
    fn test_get_damage_variants_unknown() {
        assert!(get_damage_variants("album_status").is_empty());
        assert!(get_damage_variants("special").is_empty());
    }

    #[test]
    fn test_ship_damaged_false_produces_only_base() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = make_ship_entry("banner", "this._mst_id", Some("false"));
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("ship/banner/")));
        assert!(!paths.iter().any(|p| p.contains("ship/banner_dmg/")));
        assert!(!paths.iter().any(|p| p.contains("ship/banner_g/")));
        assert!(!paths.iter().any(|p| p.contains("ship/banner_g_dmg/")));
    }

    #[test]
    fn test_ship_damaged_variable_produces_base_and_variants() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = make_ship_entry("banner", "this._mst_id", Some("_0x1a3f79"));
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("ship/banner/")));
        assert!(paths.iter().any(|p| p.contains("ship/banner_dmg/")));
        assert!(paths.iter().any(|p| p.contains("ship/banner_g/")));
        assert!(paths.iter().any(|p| p.contains("ship/banner_g_dmg/")));
        // Only 1 friendly ship (id=1), so 4 paths total (base + 3 variants)
        assert_eq!(list.items.len(), 4);
    }

    #[test]
    fn test_ship_damaged_true_produces_only_damage_variant() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        // full with damagedSource=true should produce only full_dmg
        let entry = make_ship_entry("full", "this._mst_id", Some("true"));
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        assert!(!paths.iter().any(|p| p.contains("ship/full/") && !p.contains("full_dmg")));
        assert!(paths.iter().any(|p| p.contains("ship/full_dmg/")));
    }

    #[test]
    fn test_ship_no_variants_category_unaffected() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = make_ship_entry("album_status", "this._mst_id", Some("_0x1a3f79"));
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        // No damage variants for album_status, so only base path regardless of damagedSource
        assert_eq!(list.items.len(), 1);
        assert!(list.items.iter().any(|i| i.path.contains("ship/album_status/")));
    }

    #[test]
    fn test_ship_standard_category_damaged_true_produces_only_damage_variants() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        // character_full with damagedSource=true: only damage variants, no base
        let entry = make_ship_entry("character_full", "this._mst_id", Some("true"));
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        // Should NOT contain undamaged base path
        assert!(!paths.iter().any(|p| p.contains("ship/character_full/") && !p.contains("_dmg")));
        // Should contain damage variant
        assert!(paths.iter().any(|p| p.contains("ship/character_full_dmg/")));
    }

    #[test]
    fn test_ship_standard_category_damaged_false_produces_only_base() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        // banner with damagedSource=false: only base, no variants
        let entry = make_ship_entry("banner", "this._mst_id", Some("false"));
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        // Should contain base path
        assert!(
            paths
                .iter()
                .any(|p| p.contains("ship/banner/") && !p.contains("_dmg") && !p.contains("_g"))
        );
        // Should NOT contain damage variants
        assert!(!paths.iter().any(|p| p.contains("ship/banner_dmg/")));
        assert!(!paths.iter().any(|p| p.contains("ship/banner_g/")));
    }

    #[test]
    fn test_ship_standard_category_variable_damaged_produces_base_and_variants() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        // banner with variable damagedSource: base + all variants
        let entry = make_ship_entry("banner", "this._mst_id", Some("_0x1a3f79"));
        generate_entry_paths(&entry, &mst, None, None, None, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        // Should contain base
        assert!(
            paths
                .iter()
                .any(|p| p.contains("ship/banner/") && !p.contains("_dmg") && !p.contains("_g"))
        );
        // Should contain all variants
        assert!(paths.iter().any(|p| p.contains("ship/banner_dmg/")));
        assert!(paths.iter().any(|p| p.contains("ship/banner_g_dmg/")));
        assert!(paths.iter().any(|p| p.contains("ship/banner_g/")));
        // banner has 3 variants + 1 base = 4 paths
        assert_eq!(list.items.len(), 4);
    }

    fn make_cache_rules_asset() -> CacheRulesAsset {
        CacheRulesAsset {
            version: 1,
            generated_at: "2026-04-23T00:00:00Z".to_string(),
            script_version: "6.2.8.0".to_string(),
            summary: Default::default(),
            resource_manifest: ResourceManifest {
                version: 2,
                generated_at: "2026-04-23T00:00:00Z".to_string(),
                summary: Default::default(),
                path_rules: None,
                entries: vec![],
            },
            resource_categories: ResourceCategoriesAsset::default(),
            ship_rules: CacheRuleShipRules {
                special: CacheRuleSpecialShipRule {
                    coverage_mode: ResourceCoverageMode::ObservedComplete,
                    kind: "special_cases".to_string(),
                    cases: vec![CacheRuleSpecialCase {
                        damaged: false,
                        ship_ids: vec![1],
                    }],
                    module_ids: vec!["m1".to_string()],
                    module_names: vec!["special-module".to_string()],
                },
                target_semantics: CacheRuleShipTargetSemanticsRule::default(),
            },
            slot_rules: CacheRuleSlotRules {
                item_up: CacheRuleItemUpRule {
                    coverage_mode: ResourceCoverageMode::ObservedComplete,
                    kind: "item_up_normalization".to_string(),
                    replace_map: std::collections::BTreeMap::from([
                        ("1501".to_string(), 1),
                        ("1502".to_string(), 2),
                    ]),
                    enemy_slot_border: Some(1500),
                    exclude: vec![CacheRuleExcludeEntry {
                        type_: "item_up".to_string(),
                        mst_id: 1503,
                    }],
                    module_ids: vec!["m2".to_string()],
                    module_names: vec!["slot-loader".to_string()],
                },
                btxt_flat: CacheRuleBtxtFlatRule {
                    coverage_mode: ResourceCoverageMode::ObservedComplete,
                    kind: "btxt_flat_non_enemy_runtime_slots".to_string(),
                    exclude_enemy_items: true,
                    module_ids: vec!["m3".to_string()],
                    module_names: vec!["btxt-module".to_string()],
                },
                item_up2: CacheRuleObservedSlotSubsetRule::default(),
                item_on2: CacheRuleObservedSlotSubsetRule::default(),
            },
            sound_rules: Default::default(),
            unresolved_rules: vec![],
        }
    }

    #[test]
    fn test_cache_rules_banner_semantics_limit_variants_by_scope() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = make_ship_entry("banner", "this._mst_id", Some("_0x1a3f79"));
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.resource_categories.ship_generation_groups = ShipGenerationGroups {
            default_friendly: vec!["banner".to_string()],
            default_abyssal: vec!["banner".to_string()],
            ..Default::default()
        };
        cache_rules.ship_rules.target_semantics = CacheRuleShipTargetSemanticsRule {
            coverage_mode: ResourceCoverageMode::ObservedComplete,
            kind: "ship_target_semantics".to_string(),
            cases: vec![
                CacheRuleShipTargetSemanticCase {
                    raw_target_type: "banner".to_string(),
                    selector_scope: CacheRuleShipSelectorScope::DefaultFriendly,
                    damaged_state: CacheRuleDamagedState::Variable,
                    target_types: vec!["banner".to_string(), "banner_dmg".to_string()],
                },
                CacheRuleShipTargetSemanticCase {
                    raw_target_type: "banner".to_string(),
                    selector_scope: CacheRuleShipSelectorScope::DefaultAbyssal,
                    damaged_state: CacheRuleDamagedState::Variable,
                    target_types: vec!["banner".to_string()],
                },
            ],
            module_ids: vec![],
            module_names: vec![],
        };

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert!(paths.iter().any(|path| path.contains("ship/banner/0001_")));
        assert!(paths.iter().any(|path| path.contains("ship/banner_dmg/0001_")));
        assert!(paths.iter().any(|path| path.contains("ship/banner/1500_")));
        assert!(!paths.iter().any(|path| path.contains("ship/banner_dmg/1500_")));
        assert!(!paths.iter().any(|path| path.contains("ship/banner_g/")));
        assert!(!paths.iter().any(|path| path.contains("ship/banner_g_dmg/")));
    }

    #[test]
    fn test_cache_rules_banner_g_semantics_remap_to_canonical_target() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = make_ship_entry("banner_g", "this._mst_id", Some("true"));
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.ship_rules.target_semantics = CacheRuleShipTargetSemanticsRule {
            coverage_mode: ResourceCoverageMode::ObservedComplete,
            kind: "ship_target_semantics".to_string(),
            cases: vec![CacheRuleShipTargetSemanticCase {
                raw_target_type: "banner_g".to_string(),
                selector_scope: CacheRuleShipSelectorScope::DefaultFriendly,
                damaged_state: CacheRuleDamagedState::True,
                target_types: vec!["banner_g_dmg".to_string()],
            }],
            module_ids: vec![],
            module_names: vec![],
        };

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("ship/banner_g_dmg/0001_"));
    }

    #[test]
    fn test_cache_rules_item_up2_uses_observed_subset_ids() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_slotitem = vec![
            ApiMstSlotitem {
                api_id: 1,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 525,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 526,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
        ];

        let mut list = CacheList::new();
        let entry = ResourceManifestEntry {
            kind: ManifestEntryKind::Slotitem,
            source: "test".to_string(),
            target_type: "item_up2".to_string(),
            ship_mst_id_source: None,
            damaged_source: None,
            slot_mst_id_sources: Some(vec!["_0x37d3f1".to_string()]),
            provider: None,
            texture_ids: None,
            paths: None,
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        };
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.slot_rules.item_up2 = CacheRuleObservedSlotSubsetRule {
            coverage_mode: ResourceCoverageMode::ObservedComplete,
            kind: "observed_slot_subset".to_string(),
            ids: vec![525, 526],
            module_ids: vec![],
            module_names: vec![],
        };

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|path| path.contains("slot/item_up2/0525_")));
        assert!(paths.iter().any(|path| path.contains("slot/item_up2/0526_")));
        assert!(!paths.iter().any(|path| path.contains("slot/item_up2/0001_")));
    }

    #[test]
    fn test_cache_rules_special_overrides_universal_ship_resolution() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = make_ship_entry("special", "this._mst_id", Some("false"));
        let cache_rules = make_cache_rules_asset();

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("ship/special/0001_"));
    }

    #[test]
    fn test_cache_rules_default_friendly_categories_exclude_abyssal_ship_ids() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_ship[1].api_sortno = Some(1500);
        let mut list = CacheList::new();
        let entry = make_ship_entry("album_status", "this._mst_id", Some("false"));
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.resource_categories.ship_generation_groups = ShipGenerationGroups {
            default_friendly: vec!["album_status".to_string()],
            ..Default::default()
        };

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("ship/album_status/0001_"));
        assert!(!paths.iter().any(|path| path.contains("ship/album_status/1500_")));
    }

    #[test]
    fn test_cache_rules_default_abyssal_categories_exclude_friendly_ship_ids() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_ship[1].api_sortno = Some(1500);
        let mut list = CacheList::new();
        let entry = make_ship_entry("banner3", "this._mst_id", Some("false"));
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.resource_categories.ship_generation_groups = ShipGenerationGroups {
            default_abyssal: vec!["banner3".to_string()],
            ..Default::default()
        };

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 1);
        assert!(paths.iter().any(|path| path.contains("ship/banner3/1500_")));
        assert!(!paths.iter().any(|path| path.contains("ship/banner3/0001_")));
    }

    #[test]
    fn test_cache_rules_special_prefers_manifest_path_rule_ids_when_available() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_ship.push(ApiMstShip {
            api_id: 2,
            api_sortno: Some(2),
            api_aftershipid: Some("3".to_string()),
            api_name: "SecondShip".to_string(),
            ..Default::default()
        });
        mst.api_mst_shipgraph.push(ApiMstShipgraph {
            api_id: 2,
            api_sortno: Some(2),
            api_filename: "2".to_string(),
            api_version: vec!["1".to_string()],
            ..Default::default()
        });

        let mut list = CacheList::new();
        let entry = make_ship_entry("special", "this._mst_id", Some("false"));
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.resource_manifest.path_rules = Some(PathRules {
            special_ships: vec![1, 2],
            ..Default::default()
        });

        generate_entry_paths(
            &entry,
            &mst,
            cache_rules.resource_manifest.path_rules.as_ref(),
            None,
            Some(&cache_rules),
            &mut list,
        );

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|path| path.contains("ship/special/0001_")));
        assert!(paths.iter().any(|path| path.contains("ship/special/0002_")));
    }

    #[test]
    fn test_cache_rules_character_full_uses_graph_selectors_and_event_holes() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_ship[1].api_sortno = Some(1500);
        mst.api_mst_ship.push(ApiMstShip {
            api_id: 2,
            api_sortno: Some(2),
            api_aftershipid: Some("3".to_string()),
            api_name: "NoGraphVersion".to_string(),
            ..Default::default()
        });
        mst.api_mst_shipgraph.push(ApiMstShipgraph {
            api_id: 2,
            api_sortno: Some(2),
            api_filename: "2".to_string(),
            api_version: vec![],
            ..Default::default()
        });
        mst.api_mst_ship.push(ApiMstShip {
            api_id: 6000,
            api_sortno: None,
            api_aftershipid: Some("6001".to_string()),
            api_name: "EventShip".to_string(),
            ..Default::default()
        });
        mst.api_mst_shipgraph.push(ApiMstShipgraph {
            api_id: 6000,
            api_sortno: None,
            api_filename: "6000".to_string(),
            api_version: vec![],
            ..Default::default()
        });
        mst.api_mst_ship.push(ApiMstShip {
            api_id: 6001,
            api_sortno: None,
            api_aftershipid: Some("6002".to_string()),
            api_name: "EventHole".to_string(),
            ..Default::default()
        });
        mst.api_mst_shipgraph.push(ApiMstShipgraph {
            api_id: 6001,
            api_sortno: None,
            api_filename: "6001".to_string(),
            api_version: vec![],
            ..Default::default()
        });

        let mut list = CacheList::new();
        let entry = make_ship_entry("character_full", "this._mst_id", Some("false"));
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.resource_categories.ship_generation_groups = ShipGenerationGroups {
            friend_graph: vec!["character_full".to_string()],
            ..Default::default()
        };
        cache_rules.resource_manifest.path_rules = Some(PathRules {
            event_ship_holes: ShipPathHoles {
                full: vec![6001],
                ..Default::default()
            },
            ..Default::default()
        });

        generate_entry_paths(
            &entry,
            &mst,
            cache_rules.resource_manifest.path_rules.as_ref(),
            None,
            Some(&cache_rules),
            &mut list,
        );

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|path| path.contains("ship/character_full/0001_")));
        assert!(paths.iter().any(|path| path.contains("ship/character_full/6000_")));
        assert!(!paths.iter().any(|path| path.contains("ship/character_full/0002_")));
        assert!(!paths.iter().any(|path| path.contains("ship/character_full/1500_")));
        assert!(!paths.iter().any(|path| path.contains("ship/character_full/6001_")));
    }

    #[test]
    fn test_cache_rules_full_dmg_uses_graph_selectors_and_enemy_holes() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_ship[1].api_sortno = Some(1500);
        mst.api_mst_shipgraph[1].api_version = vec!["1".to_string()];
        mst.api_mst_ship.push(ApiMstShip {
            api_id: 1501,
            api_sortno: Some(1501),
            api_aftershipid: None,
            api_name: "EnemyHole".to_string(),
            ..Default::default()
        });
        mst.api_mst_shipgraph.push(ApiMstShipgraph {
            api_id: 1501,
            api_sortno: None,
            api_filename: "1501".to_string(),
            api_version: vec!["1".to_string()],
            ..Default::default()
        });
        mst.api_mst_ship.push(ApiMstShip {
            api_id: 6000,
            api_sortno: None,
            api_aftershipid: Some("6001".to_string()),
            api_name: "EventShip".to_string(),
            ..Default::default()
        });
        mst.api_mst_shipgraph.push(ApiMstShipgraph {
            api_id: 6000,
            api_sortno: None,
            api_filename: "6000".to_string(),
            api_version: vec!["1".to_string()],
            ..Default::default()
        });

        let mut list = CacheList::new();
        let entry = make_ship_entry("full", "this._mst_id", Some("true"));
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.resource_categories.ship_generation_groups = ShipGenerationGroups {
            friend_graph: vec!["full".to_string()],
            enemy_graph: vec!["full".to_string()],
            ..Default::default()
        };
        cache_rules.resource_manifest.path_rules = Some(PathRules {
            enemy_ship_holes: ShipPathHoles {
                full_dmg: vec![1501],
                ..Default::default()
            },
            ..Default::default()
        });

        generate_entry_paths(
            &entry,
            &mst,
            cache_rules.resource_manifest.path_rules.as_ref(),
            None,
            Some(&cache_rules),
            &mut list,
        );

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|path| path.contains("ship/full_dmg/0001_")));
        assert!(paths.iter().any(|path| path.contains("ship/full_dmg/1500_")));
        assert!(!paths.iter().any(|path| path.contains("ship/full_dmg/1501_")));
        assert!(!paths.iter().any(|path| path.contains("ship/full_dmg/6000_")));
    }

    #[test]
    fn test_cache_rules_item_up_normalizes_enemy_slot_ids() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_slotitem = vec![
            ApiMstSlotitem {
                api_id: 1,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 2,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 1501,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 1502,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 1503,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
        ];
        let mut list = CacheList::new();
        let cache_rules = make_cache_rules_asset();
        let entry = ResourceManifestEntry {
            kind: ManifestEntryKind::Slotitem,
            source: "test".to_string(),
            target_type: "item_up".to_string(),
            ship_mst_id_source: None,
            damaged_source: None,
            slot_mst_id_sources: Some(vec!["this._slot1.mstID".to_string()]),
            provider: None,
            texture_ids: None,
            paths: None,
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        };

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert!(paths.iter().any(|path| path.contains("slot/item_up/0001_")));
        assert!(paths.iter().any(|path| path.contains("slot/item_up/0002_")));
        assert!(!paths.iter().any(|path| path.contains("slot/item_up/1501_")));
        assert!(!paths.iter().any(|path| path.contains("slot/item_up/1503_")));
    }

    #[test]
    fn test_cache_rules_btxt_flat_limits_to_non_enemy_slot_ids() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_slotitem = vec![
            ApiMstSlotitem {
                api_id: 1,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 2,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 1501,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
        ];
        let mut list = CacheList::new();
        let cache_rules = make_cache_rules_asset();
        let entry = ResourceManifestEntry {
            kind: ManifestEntryKind::Slotitem,
            source: "test".to_string(),
            target_type: "btxt_flat".to_string(),
            ship_mst_id_source: None,
            damaged_source: None,
            slot_mst_id_sources: Some(vec!["this._slot1.mstID".to_string()]),
            provider: None,
            texture_ids: None,
            paths: None,
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        };

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert!(paths.iter().any(|path| path.contains("slot/btxt_flat/0001_")));
        assert!(paths.iter().any(|path| path.contains("slot/btxt_flat/0002_")));
        assert!(!paths.iter().any(|path| path.contains("slot/btxt_flat/1501_")));
    }

    #[test]
    fn test_cache_rules_btxt_flat_prefers_manifest_path_rule_ids_when_available() {
        let mut mst = make_minimal_manifest();
        mst.api_mst_slotitem = vec![
            ApiMstSlotitem {
                api_id: 1,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 2,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
            ApiMstSlotitem {
                api_id: 1501,
                api_sortno: 1,
                api_version: Some(1),
                ..Default::default()
            },
        ];
        let mut list = CacheList::new();
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.resource_manifest.path_rules = Some(PathRules {
            btxt_flat_slot_ids: vec![2],
            ..Default::default()
        });
        let entry = ResourceManifestEntry {
            kind: ManifestEntryKind::Slotitem,
            source: "test".to_string(),
            target_type: "btxt_flat".to_string(),
            ship_mst_id_source: None,
            damaged_source: None,
            slot_mst_id_sources: Some(vec!["this._slot1.mstID".to_string()]),
            provider: None,
            texture_ids: None,
            paths: None,
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        };

        generate_entry_paths(
            &entry,
            &mst,
            cache_rules.resource_manifest.path_rules.as_ref(),
            None,
            Some(&cache_rules),
            &mut list,
        );

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].contains("slot/btxt_flat/0002_"));
    }

    #[test]
    fn test_sortno_zero_excluded_from_friend_graph() {
        let mut mst = make_minimal_manifest();
        // Ship with sortno=Some(0) — graph-only entry, not a real playable ship
        mst.api_mst_shipgraph.push(ApiMstShipgraph {
            api_id: 3,
            api_sortno: Some(0),
            api_filename: "3".to_string(),
            api_version: vec!["1".to_string()],
            ..Default::default()
        });

        let mut list = CacheList::new();
        let entry = make_ship_entry("character_full", "this._mst_id", Some("false"));
        let mut cache_rules = make_cache_rules_asset();
        cache_rules.resource_categories.ship_generation_groups = ShipGenerationGroups {
            friend_graph: vec!["character_full".to_string()],
            ..Default::default()
        };

        generate_entry_paths(&entry, &mst, None, None, Some(&cache_rules), &mut list);

        let paths = list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        // sortno=0 ship (ID 3) must NOT produce character_full paths
        assert!(
            !paths.iter().any(|path| path.contains("ship/character_full/0003_")),
            "sortno=0 entries should be excluded from friend_graph targets"
        );
        // Regular friendly ship (ID 1, sortno=Some(1)) should still produce paths
        assert!(paths.iter().any(|path| path.contains("ship/character_full/0001_")));
    }
}
