use std::sync::{LazyLock, Mutex};

use emukc_crypto::SuffixUtils;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
    make_list::CacheList,
    make_list::manifest::{PathRules, ResourceCategoriesAsset, ShipPathHoles},
};

static HOLES_COLLECTOR: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

pub fn get_holes_report() -> Vec<String> {
    HOLES_COLLECTOR.lock().unwrap().clone()
}

pub fn clear_holes_report() {
    HOLES_COLLECTOR.lock().unwrap().clear();
}

fn has_ship_holes(holes: &ShipPathHoles) -> bool {
    !holes.full.is_empty()
        || !holes.full_dmg.is_empty()
        || !holes.up.is_empty()
        || !holes.up_dmg.is_empty()
}

fn select_holes<'a>(
    rules_holes: Option<&'a ShipPathHoles>,
    fallback: &'a ShipPathHoles,
) -> &'a ShipPathHoles {
    rules_holes.filter(|holes| has_ship_holes(holes)).unwrap_or(fallback)
}

#[cfg(test)]
fn select_ids<'a>(rules_ids: Option<&'a [i64]>, fallback: &'a [i64]) -> &'a [i64] {
    rules_ids.filter(|ids| !ids.is_empty()).unwrap_or(fallback)
}

static EVENT_SHIP_HOLES: LazyLock<ShipPathHoles> = LazyLock::new(|| ShipPathHoles {
    full: vec![
        5358, 5514, 5526, 5527, 5530, 5531, 5532, 5534, 5536, 5848, 5849, 5850, 5851, 5852, 5853,
        6233, 6234, 6235, 6236, 6237, 6238, 6239, 6240, 6241, 6242,
    ],
    full_dmg: vec![
        5026, 5027, 5275, 5276, 5277, 5278, 5279, 5280, 5281, 5282, 5283, 5284, 5285, 5286, 5287,
        5288, 5289, 5290, 5291, 5292, 5293, 5294, 5295, 5296, 5297, 5298, 5299, 5300, 5301, 5302,
        5303, 5304, 5305, 5306, 5358, 5514, 5526, 5527, 5530, 5531, 5532, 5534, 5536, 5667, 5668,
        5669, 5848, 5849, 5850, 5851, 5852, 5853, 6233, 6234, 6235, 6236, 6237, 6238, 6239, 6240,
        6241, 6242,
    ],
    up: vec![
        5358, 5514, 5526, 5527, 5530, 5531, 5532, 5534, 5536, 5848, 5849, 5850, 5851, 5852, 5853,
        6233, 6234, 6235, 6236, 6237, 6238, 6239, 6240, 6241, 6242,
    ],
    up_dmg: vec![
        5026, 5027, 5275, 5276, 5277, 5278, 5279, 5280, 5281, 5282, 5283, 5284, 5285, 5286, 5287,
        5288, 5289, 5290, 5291, 5292, 5293, 5294, 5295, 5296, 5297, 5298, 5299, 5300, 5301, 5302,
        5303, 5304, 5305, 5306, 5358, 5514, 5526, 5527, 5530, 5531, 5532, 5534, 5536, 5667, 5668,
        5669, 5848, 5849, 5850, 5851, 5852, 5853, 6233, 6234, 6235, 6236, 6237, 6238, 6239, 6240,
        6241, 6242,
    ],
});

pub(crate) fn make_manifest_category_extensions(
    mst: &ApiManifest,
    list: &mut CacheList,
    rules: Option<&PathRules>,
    categories: Option<&ResourceCategoriesAsset>,
) {
    let Some(categories) = categories else {
        return;
    };

    if categories
        .ship_generation_groups
        .default_friendly
        .iter()
        .any(|category| category == "power_up")
    {
        for ship in mst.api_mst_ship.iter().filter(|ship| ship.api_aftershipid.is_some()) {
            let ship_id = format!("{:04}", ship.api_id);
            let version = mst
                .api_mst_shipgraph
                .iter()
                .find(|graph| graph.api_id == ship.api_id)
                .and_then(|graph| graph.api_version.first());
            list.add(
                format!(
                    "kcs2/resources/ship/power_up/{ship_id}_{}.png",
                    SuffixUtils::create(&ship_id, "ship_power_up")
                ),
                version,
            );
        }
    }

    let has_character_graphs =
        categories.ship_generation_groups.friend_graph.iter().any(|category| {
            matches!(
                category.as_str(),
                "character_full" | "character_full_dmg" | "character_up" | "character_up_dmg"
            )
        });
    if has_character_graphs {
        make_friend_event_graph_with_rules(mst, list, rules);
    }

    if categories.sp_remodel_subcategories.iter().any(|category| category == "animation_key") {
        let ship_ids = rules
            .filter(|rules| !rules.sp_remodel_ships.is_empty())
            .map(|rules| rules.sp_remodel_ships.as_slice())
            .unwrap_or(SP_REMODEL_SHIPS.as_slice());
        for id in ship_ids {
            list.add(
                format!("kcs2/resources/ship/sp_remodel/animation_key/{id:04}_remodel.json"),
                mst.find_shipgraph(*id).and_then(|graph| graph.api_version.first()),
            );
        }
    }
}

pub(crate) fn make_manifest_type_extensions(mst: &ApiManifest, list: &mut CacheList) {
    make_ship_type(mst, list);
}

fn make_friend_event_graph_with_rules(
    mst: &ApiManifest,
    list: &mut CacheList,
    rules: Option<&PathRules>,
) {
    let holes = select_holes(rules.map(|rules| &rules.event_ship_holes), &EVENT_SHIP_HOLES);
    for graph in mst.api_mst_shipgraph.iter() {
        if graph.api_id < 5000 {
            continue;
        }

        let ship_id = format!("{0:04}", graph.api_id);

        let mut categories: Vec<String> = Vec::new();

        if !holes.full.contains(&graph.api_id) {
            categories.push("character_full".to_string());
        }

        if !holes.full_dmg.contains(&graph.api_id) {
            categories.push("character_full_dmg".to_string());
        }

        if !holes.up.contains(&graph.api_id) {
            categories.push("character_up".to_string());
        }

        if !holes.up_dmg.contains(&graph.api_id) {
            categories.push("character_up_dmg".to_string());
        }

        for category in categories.iter() {
            list.add(
                format!(
                    "kcs2/resources/ship/{category}/{ship_id}_{}.png",
                    SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
                ),
                graph.api_version.first(),
            );
        }
    }
}

#[cfg(test)]
static ENEMY_SHIP_HOLES: LazyLock<ShipPathHoles> = LazyLock::new(|| ShipPathHoles {
    full: vec![1563, 1568, 1569, 1580, 1593, 1596],
    full_dmg: vec![
        1556, 1557, 1563, 1568, 1569, 1580, 1593, 1596, 1650, 1651, 1652, 1673, 1674, 1675, 1679,
        1680, 1681, 1682, 1683, 1684, 1685, 1686, 1687, 1688, 1689, 1690, 1691, 1692, 1846, 1847,
        1848, 1849, 1850, 1851, 1852, 1853, 1854, 1855, 1856, 1857, 1858, 1859, 1860, 1861, 1862,
        1863, 1864, 1865, 1866, 1867, 1868, 1869, 1870, 1871, 1872, 1873, 1874, 1875, 1876, 1877,
        1878, 1879, 1880, 1881, 1882, 1883, 1884, 1885, 1886, 1887, 1888, 1889, 1890, 1891, 1892,
        1893, 1894, 1895, 1896, 1897, 1898, 1899, 1900, 1901, 1902, 1903, 1904, 1905, 1906, 1907,
        1908, 1909, 1910, 1911, 1912, 1913, 1914, 1915, 1916, 1917, 1918, 1919, 1920, 2063, 2064,
        2065, 2066, 2067, 2068, 2069, 2070, 2071, 2072, 2073, 2074, 2075, 2076, 2077, 2078, 2079,
        2080, 2081, 2082, 2083, 2084, 2085, 2086, 2087, 2088, 2089, 2090, 2094, 2095, 2096,
    ],
    up: vec![],
    up_dmg: vec![],
});

#[cfg(test)]
fn make_enemy_graph_with_rules(mst: &ApiManifest, list: &mut CacheList, rules: Option<&PathRules>) {
    let holes = select_holes(rules.map(|rules| &rules.enemy_ship_holes), &ENEMY_SHIP_HOLES);
    for graph in mst.api_mst_shipgraph.iter() {
        if graph.api_sortno.is_some() || graph.api_id >= 5000 {
            continue;
        }

        let ship_id = format!("{0:04}", graph.api_id);

        let mut categories: Vec<String> = Vec::new();

        if !holes.full.contains(&graph.api_id) {
            categories.push("full".to_string());
        }

        if !holes.full_dmg.contains(&graph.api_id) {
            categories.push("full_dmg".to_string());
        }

        for category in categories.iter() {
            list.add(
                format!(
                    "kcs2/resources/ship/{category}/{ship_id}_{}_{}.png",
                    SuffixUtils::create(&ship_id, format!("ship_{category}").as_str()),
                    graph.api_filename
                ),
                graph.api_version.first(),
            );
        }
    }
}

#[cfg(test)]
static SPECIAL_SHIPS: LazyLock<Vec<i64>> = LazyLock::new(|| {
    vec![
        639, 724, 694, 969, 918, 446, 554, 553, 576, 411, 184, 733, 412, 944, 916, 634, 577, 592,
        572, 949, 392, 571, 635, 573, 640, 447, 541, 1496, 659, 601, 697, 546, 911, 364, 178, 360,
        954, 591, 913, 593,
    ]
});

#[cfg(test)]
fn make_ship_special_with_rules(
    mst: &ApiManifest,
    list: &mut CacheList,
    rules: Option<&PathRules>,
) {
    let special_ships =
        select_ids(rules.map(|rules| rules.special_ships.as_slice()), SPECIAL_SHIPS.as_slice());

    special_ships.iter().filter_map(|id| mst.find_shipgraph(*id)).for_each(|graph| {
        let ship_id = format!("{0:04}", graph.api_id);
        let p = format!(
            "kcs2/resources/ship/special/{ship_id}_{}.png",
            SuffixUtils::create(&ship_id, "ship_special".to_string().as_str()),
        );
        list.add(p, graph.api_version.first());
    });
}

static SP_REMODEL_SHIPS: LazyLock<Vec<i64>> = LazyLock::new(|| {
    vec![
        501, 502, 506, 507, 587, 588, 591, 592, 593, 594, 599, 610, 622, 629, 630, 646, 651, 652,
        656, 662, 663, 667, 668, 694, 698, 707, 883, 888, 894, 899, 911, 916, 951, 954, 955, 956,
        959, 960, 961, 963, 968, 969, 975, 981, 982, 983, 986, 987, 1031, 1033,
    ]
});

#[cfg(test)]
static SP_REMODEL_MES: LazyLock<Vec<i64>> = LazyLock::new(|| {
    vec![
        73, 121, 136, 145, 149, 150, 151, 152, 196, 202, 203, 204, 215, 228, 277, 278, 285, 293,
        306, 307, 316, 318, 323, 324, 325, 330, 350, 357, 369, 373, 392, 396, 501, 502, 579, 588,
        593, 594, 610, 628, 651, 663, 667, 680, 688, 698, 718, 883, 894, 911, 954, 955, 960,
    ]
});

#[cfg(test)]
fn make_sp_remodel_with_rules(mst: &ApiManifest, list: &mut CacheList, rules: Option<&PathRules>) {
    let sp_remodel_ships = select_ids(
        rules.map(|rules| rules.sp_remodel_ships.as_slice()),
        SP_REMODEL_SHIPS.as_slice(),
    );
    let sp_remodel_mes =
        select_ids(rules.map(|rules| rules.sp_remodel_mes.as_slice()), SP_REMODEL_MES.as_slice());

    for id in sp_remodel_ships.iter() {
        let Some(graph) = mst.find_shipgraph(*id) else {
            continue;
        };

        let ship_id = format!("{id:04}");
        let v = graph.api_version.first();
        let full_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/full_x2");
        let silh_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/silhouette");
        let cls_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/text_class");
        let name_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/text_name");

        list.add(format!("kcs2/resources/ship/sp_remodel/animation_key/{ship_id}_remodel.json"), v)
            .add(format!("kcs2/resources/ship/sp_remodel/full_x2/{ship_id}_{full_key}.png"), v)
            .add(format!("kcs2/resources/ship/sp_remodel/silhouette/{ship_id}_{silh_key}.png"), v)
            .add(format!("kcs2/resources/ship/sp_remodel/text_class/{ship_id}_{cls_key}.png"), v)
            .add(format!("kcs2/resources/ship/sp_remodel/text_name/{ship_id}_{name_key}.png"), v);
    }

    for id in sp_remodel_mes.iter() {
        let Some(graph) = mst.find_shipgraph(*id) else {
            continue;
        };

        let ship_id = format!("{id:04}");
        let v = graph.api_version.first();
        let mes_key = SuffixUtils::create(&ship_id, "ship_sp_remodel/text_remodel_mes");
        list.add(
            format!("kcs2/resources/ship/sp_remodel/text_remodel_mes/{ship_id}_{mes_key}.png"),
            v,
        );
    }
}

const SHIP_SP_TYPE_MAX: usize = 8;

fn make_ship_type(mst: &ApiManifest, list: &mut CacheList) {
    for stype in mst.api_mst_stype.iter() {
        if stype.api_id == 8 || stype.api_id == 15 {
            continue;
        }

        let stype_id = format!("{0:03}", stype.api_id);
        let etext = format!("kcs2/resources/stype/etext/{stype_id}.png");

        list.add(etext, "");
    }

    for i in 1..=SHIP_SP_TYPE_MAX {
        let stype_id = format!("{i:03}");
        let etext = format!("kcs2/resources/stype/etext/sp{stype_id}.png");

        list.add(etext, "");
    }
}

#[cfg(test)]
static CARD_ROUNDS: LazyLock<Vec<i64>> = LazyLock::new(|| vec![524, 525]);
#[cfg(test)]
static REWARDS: LazyLock<Vec<i64>> = LazyLock::new(|| {
    vec![
        162, 182, 183, 184, 451, 460, 491, 517, 518, 524, 525, 531, 540, 551, 552, 565, 570, 574,
        634, 635, 900, 904, 905, 943,
    ]
});

#[cfg(test)]
fn make_ship_reward_res_with_rules(
    mst: &ApiManifest,
    list: &mut CacheList,
    rules: Option<&PathRules>,
) {
    let card_rounds =
        select_ids(rules.map(|rules| rules.card_rounds.as_slice()), CARD_ROUNDS.as_slice());
    let reward_ships =
        select_ids(rules.map(|rules| rules.reward_ships.as_slice()), REWARDS.as_slice());

    for id in card_rounds.iter() {
        let Some(graph) = mst.find_shipgraph(*id) else {
            continue;
        };
        let v = graph.api_version.first().cloned().unwrap_or_default();

        let ship_id = format!("{id:04}");
        let key = SuffixUtils::create(&ship_id, "ship_card_round");
        list.add(format!("kcs2/resources/ship/card_round/{ship_id}_{key}.png"), v.clone());

        let key = SuffixUtils::create(&ship_id, "ship_icon_box");
        list.add(format!("kcs2/resources/ship/icon_box/{ship_id}_{key}.png"), v);
    }

    for id in reward_ships.iter() {
        let Some(graph) = mst.find_shipgraph(*id) else {
            continue;
        };
        let v = graph.api_version.first().cloned().unwrap_or_default();

        let ship_id = format!("{id:04}");
        let key = SuffixUtils::create(&ship_id, "ship_reward_card");
        list.add(format!("kcs2/resources/ship/reward_card/{ship_id}_{key}.png"), v.clone());

        let key = SuffixUtils::create(&ship_id, "ship_reward_icon");
        list.add(format!("kcs2/resources/ship/reward_icon/{ship_id}_{key}.png"), v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::make_list::manifest::load_resource_manifest;
    use emukc_model::kc2::start2::ApiMstShipgraph;

    fn make_graph(
        api_id: i64,
        sortno: Option<i64>,
        version: &str,
        filename: &str,
    ) -> ApiMstShipgraph {
        ApiMstShipgraph {
            api_id,
            api_sortno: sortno,
            api_version: vec![version.to_string()],
            api_filename: filename.to_string(),
            ..Default::default()
        }
    }

    fn make_manifest() -> ApiManifest {
        ApiManifest {
            api_mst_shipgraph: vec![
                make_graph(5358, Some(1), "1", "5358"),
                make_graph(1563, None, "1", "1563"),
                make_graph(639, Some(1), "1", "639"),
                make_graph(501, Some(1), "1", "501"),
                make_graph(73, Some(1), "1", "73"),
                make_graph(524, Some(1), "1", "524"),
                make_graph(525, Some(1), "1", "525"),
                make_graph(900, Some(1), "1", "900"),
            ],
            ..Default::default()
        }
    }

    fn make_rules() -> PathRules {
        load_resource_manifest()
            .unwrap()
            .path_rules
            .expect("real manifest should include pathRules")
    }

    #[test]
    fn test_full() {
        let ship_id = format!("{0:04}", 1);
        let key = SuffixUtils::create(&ship_id, "ship_full_dmg".to_string().as_str());

        assert_eq!(key, "6245");
    }

    #[test]
    fn test_sp_remodel() {
        // vec![
        // 			"/animation_key/0502_remodel.json",
        // 			"/full_x2/0502_8686.png",
        // 			"/silhouette/0502_8686.png",
        // 			"/text_class/0502_8209.png",
        // 			"/text_name/0502_4089.png",
        // 		];
        let ship_id = "0121";
        for category in ["full_x2", "silhouette", "text_class", "text_name", "text_remodel_mes"] {
            println!(
                "{}",
                SuffixUtils::create(ship_id, format!("ship_sp_remodel/{}", category).as_str())
            );
        }
    }

    #[test]
    fn test_extra() {
        let ship_id = format!("{0:04}", 5808);
        for category in ["character_full", "character_full_dmg", "character_up", "character_up_dmg"]
        {
            let key = SuffixUtils::create(&ship_id, format!("ship_{}", category).as_str());
            println!(
                "http://w01y.kancolle-server.com/kcs2/resources/ship/{category}/{ship_id}_{}.png",
                key
            );
        }
    }

    #[test]
    fn test_real_manifest_path_rules_match_ship_constants() {
        let rules = make_rules();
        assert_eq!(rules.event_ship_holes, EVENT_SHIP_HOLES.clone());
        assert_eq!(rules.enemy_ship_holes, ENEMY_SHIP_HOLES.clone());
        assert_eq!(rules.special_ships, SPECIAL_SHIPS.to_vec());
        assert_eq!(rules.sp_remodel_ships, SP_REMODEL_SHIPS.to_vec());
        assert_eq!(rules.sp_remodel_mes, SP_REMODEL_MES.to_vec());
        assert_eq!(rules.card_rounds, CARD_ROUNDS.to_vec());
        assert_eq!(rules.reward_ships, REWARDS.to_vec());
    }

    #[test]
    fn test_default_ship_outputs_match_with_and_without_path_rules() {
        let mst = make_manifest();
        let rules = make_rules();

        let mut fallback_list = CacheList::new();
        make_friend_event_graph_with_rules(&mst, &mut fallback_list, None);
        make_enemy_graph_with_rules(&mst, &mut fallback_list, None);
        make_ship_special_with_rules(&mst, &mut fallback_list, None);
        make_sp_remodel_with_rules(&mst, &mut fallback_list, None);
        make_ship_reward_res_with_rules(&mst, &mut fallback_list, None);

        let mut rule_list = CacheList::new();
        make_friend_event_graph_with_rules(&mst, &mut rule_list, Some(&rules));
        make_enemy_graph_with_rules(&mst, &mut rule_list, Some(&rules));
        make_ship_special_with_rules(&mst, &mut rule_list, Some(&rules));
        make_sp_remodel_with_rules(&mst, &mut rule_list, Some(&rules));
        make_ship_reward_res_with_rules(&mst, &mut rule_list, Some(&rules));

        let fallback_paths =
            fallback_list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        let rule_paths = rule_list.items.iter().map(|item| item.path.as_str()).collect::<Vec<_>>();
        assert_eq!(rule_paths, fallback_paths);
    }
}
