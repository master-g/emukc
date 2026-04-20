use emukc_cache::IntoVersion;
use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

use super::resolve;
use super::types::{ManifestEntryKind, ResourceManifestEntry};
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

pub(crate) fn generate_entry_paths(
    entry: &ResourceManifestEntry,
    mst: &ApiManifest,
    list: &mut CacheList,
) {
    match entry.kind {
        ManifestEntryKind::Ship => generate_ship_paths(entry, mst, list),
        ManifestEntryKind::Slotitem => generate_slotitem_paths(entry, mst, list),
        ManifestEntryKind::ExplicitPath => generate_explicit_paths(entry, list),
        ManifestEntryKind::TextureProvider => {
            // Deferred to future phase
        }
        ManifestEntryKind::Unknown(ref k) => {
            warn!("Skipping unknown manifest entry kind: {k}");
        }
    }
}

fn generate_ship_paths(entry: &ResourceManifestEntry, mst: &ApiManifest, list: &mut CacheList) {
    let Some(ref source) = entry.ship_mst_id_source else {
        return;
    };

    let ship_ids = resolve::resolve_ship_ids(source, mst);
    if ship_ids.is_empty() {
        return;
    }

    let damaged = entry.damaged_source.as_deref().and_then(resolve::resolve_damaged);

    let target = entry.target_type.as_str();
    let variants = get_damage_variants(target);

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

        // Check if this is a full/full_dmg category (uses api_filename)
        if SHIP_FULL_CATEGORIES.contains(&target) {
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
        if SHIP_STANDARD_CATEGORIES.contains(&target) {
            let gen_base = !matches!(damaged, Some(true));
            let gen_variants = damaged.is_none() && !variants.is_empty();

            // Generate base path
            if gen_base {
                let suffix = SuffixUtils::create(&ship_id, format!("ship_{target}").as_str());
                list.add(
                    format!("kcs2/resources/ship/{target}/{ship_id}_{suffix}.png"),
                    version.as_ref(),
                );
            }

            // Generate damage variant paths
            if gen_variants {
                for variant in variants {
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

fn generate_slotitem_paths(entry: &ResourceManifestEntry, mst: &ApiManifest, list: &mut CacheList) {
    let sources = entry.slot_mst_id_sources.as_deref().unwrap_or(&[]);
    let slot_ids = resolve::resolve_slotitem_ids(sources, mst);
    if slot_ids.is_empty() {
        return;
    }

    let target = entry.target_type.as_str();

    for id in slot_ids {
        let item_id = format!("{id:04}");

        let version =
            mst.api_mst_slotitem.iter().find(|s| s.api_id == id).and_then(|s| s.api_version);

        if SLOT_STANDARD_CATEGORIES.contains(&target) {
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
            damaged_source: damaged.map(|s| s.to_string()),
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
        generate_entry_paths(&entry, &mst, &mut list);

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
        generate_entry_paths(&entry, &mst, &mut list);

        assert!(!list.items.is_empty());
        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.iter().any(|p| p.contains("slot/card/")));
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
                "resources/ship/".to_string(),                 // directory — skip
                "resources/stype/etext/sp001.png".to_string(), // file — include
            ]),
            module_ids: vec![],
            module_names: vec![],
            other: Default::default(),
        };
        generate_entry_paths(&entry, &mst, &mut list);

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
        generate_entry_paths(&entry, &mst, &mut list);

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
        generate_entry_paths(&entry, &mst, &mut list);

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
        generate_entry_paths(&entry, &mst, &mut list);

        let paths: Vec<&str> = list.items.iter().map(|i| i.path.as_str()).collect();
        assert!(!paths.iter().any(|p| p.contains("ship/full/") && !p.contains("full_dmg")));
        assert!(paths.iter().any(|p| p.contains("ship/full_dmg/")));
    }

    #[test]
    fn test_ship_no_variants_category_unaffected() {
        let mst = make_minimal_manifest();
        let mut list = CacheList::new();
        let entry = make_ship_entry("album_status", "this._mst_id", Some("_0x1a3f79"));
        generate_entry_paths(&entry, &mst, &mut list);

        // No damage variants for album_status, so only base path regardless of damagedSource
        assert_eq!(list.items.len(), 1);
        assert!(list.items.iter().any(|i| i.path.contains("ship/album_status/")));
    }
}
