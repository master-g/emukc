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
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	match strategy {
		CacheListMakeStrategy::Default => {
			make_useitem(list);
		}
		CacheListMakeStrategy::Greedy(concurrent) => {
			make_useitem_greedy(mst, cache, concurrent, list).await?;
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
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, v), exists) in check_result {
		if exists {
			println!("{}, {}", p, v);
			list.add(p, v);
		}
	}

	Ok(())
}

const CARD_IDS: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		001, 003, 004, 005, 011, 012, 031, 032, 033, 034, 049, 051, 052, 054, 055, 056, 057, 058,
		059, 060, 061, 062, 063, 064, 065, 066, 067, 068, 069, 070, 071, 072, 073, 074, 075, 077,
		078, 079, 080, 081, 082, 083, 084, 085, 086, 087, 088, 089, 090, 091, 092, 093, 094, 095,
		096, 097, 098, 099, 100, 101, 102,
	]
});

const CARD_UNDERLINE_IDS: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		001, 002, 003, 004, 005, 011, 012, 031, 032, 033, 034, 044, 049, 051, 052, 054, 057, 058,
		059, 060, 064, 065, 068, 070, 071, 073, 074, 075, 077, 078, 090, 091, 092, 094, 095, 096,
		097, 098, 099, 100, 101,
	]
});

fn make_useitem(list: &mut CacheList) {
	for id in CARD_IDS.iter() {
		let p = format!("kcs2/resources/useitem/card/{0:03}.png", id);
		list.add(p, "".to_string());
	}
	for id in CARD_UNDERLINE_IDS.iter() {
		let p = format!("kcs2/resources/useitem/card_/{0:03}.png", id);
		list.add(p, "".to_string());
	}
}
