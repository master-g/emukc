use std::sync::{Arc, LazyLock};

use emukc_cache::Kache;

use crate::{
	make_list::{CacheList, batch_check_exists},
	prelude::{CacheListMakeStrategy, CacheListMakingError},
};

pub(super) async fn make(
	_cache: &Kache,
	_strategy: CacheListMakeStrategy,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	make_preset(list);

	// match strategy {
	// 	CacheListMakeStrategy::Default => {
	// 		make_preset(list);
	// 	}
	// 	CacheListMakeStrategy::Greedy(concurrent) => {
	// 		// too slow
	// 		make_greedy(cache, concurrent, list).await?;
	// 	}
	// };

	Ok(())
}

#[allow(unused)]
async fn make_greedy(
	cache: &Kache,
	concurrent: usize,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	let checks: Vec<(String, String)> =
		(428..=2000).map(|v| (format!("kcs/sound/kc9997/{v}.mp3"), "".to_string())).collect();

	let c = Arc::new(cache.clone());
	let check_result = batch_check_exists(c, checks, concurrent).await?;

	for ((p, _), exists) in check_result {
		if exists {
			println!("{}", p);
			list.add_unversioned(p);
		}
	}
	Ok(())
}

const ID: LazyLock<Vec<i64>> = LazyLock::new(|| vec![428, 1186, 1871, 1188, 1187]);

fn make_preset(list: &mut CacheList) {
	for i in ID.iter() {
		let p = format!("kcs/sound/kc9997/{i}.mp3");
		list.add_unversioned(p);
	}
}
