use std::sync::{Arc, LazyLock};

use emukc_cache::Kache;
use emukc_model::kc2::start2::ApiManifest;

use crate::{
    make_list::{CacheList, batch_check_exists},
    prelude::{CacheListMakeStrategy, CacheListMakingError},
};

pub(super) async fn make(
    mst: &ApiManifest,
    cache: &Kache,
    strategy: &CacheListMakeStrategy,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    match strategy {
        CacheListMakeStrategy::Default
        | CacheListMakeStrategy::Minimal
        | CacheListMakeStrategy::Manifest
        | CacheListMakeStrategy::Rules => {
            make_useitem(list);
        }
        CacheListMakeStrategy::Greedy(config) => {
            make_useitem_greedy(mst, cache, config.concurrent, list).await?;
        }
    };

    Ok(())
}

async fn make_useitem_greedy(
    mst: &ApiManifest,
    cache: &Kache,
    concurrent: usize,
    list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
    let checks: Vec<(String, String)> = mst
        .api_mst_useitem
        .iter()
        .flat_map(|item| {
            vec![
                (format!("kcs2/resources/useitem/card/{0:03}.png", item.api_id), "".to_string()),
                (format!("kcs2/resources/useitem/card_/{0:03}.png", item.api_id), "".to_string()),
            ]
        })
        .collect();

    let c = Arc::new(cache.clone());
    let tracker = Arc::new(crate::make_list::progress::ProgressTracker::new(checks.len()));
    let check_result = batch_check_exists(c, checks, concurrent, Some(tracker)).await?;

    for ((p, v), exists) in check_result {
        if exists {
            println!("{p}, {v}");
            list.add(p, v);
        }
    }

    Ok(())
}

static CARD_IDS: LazyLock<Vec<i64>> = LazyLock::new(|| {
    vec![
        1, 3, 4, 5, 11, 12, 31, 32, 33, 34, 49, 51, 52, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64,
        65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88,
        89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102,
    ]
});

static CARD_UNDERLINE_IDS: LazyLock<Vec<i64>> = LazyLock::new(|| {
    vec![
        1, 2, 3, 4, 5, 11, 12, 31, 32, 33, 34, 44, 49, 51, 52, 54, 57, 58, 59, 60, 64, 65, 68, 70,
        71, 73, 74, 75, 77, 78, 90, 91, 92, 94, 95, 96, 97, 98, 99, 100, 101,
    ]
});

pub(super) fn is_default_card_id(id: &str) -> bool {
    parse_id(id).is_some_and(|id| CARD_IDS.contains(&id))
}

pub(super) fn is_default_underline_id(id: &str) -> bool {
    parse_id(id).is_some_and(|id| CARD_UNDERLINE_IDS.contains(&id))
}

fn parse_id(id: &str) -> Option<i64> {
    id.parse::<i64>().ok()
}

fn make_useitem(list: &mut CacheList) {
    for id in CARD_IDS.iter() {
        let p = format!("kcs2/resources/useitem/card/{id:03}.png");
        list.add(p, "".to_string());
    }
    for id in CARD_UNDERLINE_IDS.iter() {
        let p = format!("kcs2/resources/useitem/card_/{id:03}.png");
        list.add(p, "".to_string());
    }
}
