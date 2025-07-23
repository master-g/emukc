use std::sync::{Arc, LazyLock};

use emukc_cache::Kache;

use crate::{
	make_list::{CacheList, batch_check_exists},
	prelude::{CacheListMakeStrategy, CacheListMakingError},
};

pub(super) async fn make(
	cache: &Kache,
	strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	match strategy {
		CacheListMakeStrategy::Default => {
			make_preset(list);
		}
		CacheListMakeStrategy::Minimal => {
			return Ok(());
		}
		CacheListMakeStrategy::Greedy(concurrent) => {
			make_greedy(cache, concurrent, list).await?;
		}
	};

	Ok(())
}

#[allow(unused)]
async fn make_greedy(
	cache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let checks: Vec<(String, String)> =
		(1..=2000).map(|v| (format!("kcs/sound/kc9999/{v}.mp3"), "".to_string())).collect();

	let c = Arc::new(cache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, _), exists) in check_result {
		if exists {
			println!("{p}");
			list.add_unversioned(p);
		}
	}
	Ok(())
}

static ID: LazyLock<Vec<i64>> = LazyLock::new(|| {
	vec![
		1, 2, 3, 11, 12, 13, 14, 15, 16, 17, 18, 21, 22, 23, 24, 25, 26, 27, 28, 37, 38, 301, 302,
		303, 304, 305, 306, 307, 308, 309, 310, 311, 312, 313, 314, 315, 316, 317, 318, 319, 320,
		321, 322, 323, 324, 325, 326, 327, 328, 329, 330, 331, 332, 333, 334, 335, 336, 337, 338,
		339, 340, 341, 342, 343, 344, 401, 402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412,
		413, 414, 415, 416, 417, 418, 419, 420, 421, 422, 423, 424, 425, 426, 427, 428, 429, 430,
		431, 432, 433, 434, 435, 436, 1101, 1158, 1186, 1187, 1188, 1189, 1190, 1191, 1193, 1195,
		1198, 1871, 1877,
	]
});

fn make_preset(list: &mut CacheList) {
	for i in ID.iter() {
		let p = format!("kcs/sound/kc9999/{i}.mp3");
		list.add_unversioned(p);
	}
}
