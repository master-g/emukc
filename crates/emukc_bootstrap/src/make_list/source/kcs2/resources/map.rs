use std::sync::LazyLock;

use emukc_cache::{Kache, NoVersion};

use crate::{make_list::CacheList, prelude::CacheListMakingError};

static DEFAULT_AREAS: LazyLock<Vec<i64>> = LazyLock::new(|| vec![1, 2, 3, 4, 5, 6, 7]);

pub(super) async fn make(cache: &Kache, list: &mut CacheList) -> Result<(), CacheListMakingError> {
	get_default_areas(cache, list).await?;
	Ok(())
}

async fn get_default_areas(
	cache: &Kache,
	list: &mut CacheList,
) -> Result<(), CacheListMakingError> {
	for area_id in DEFAULT_AREAS.iter() {
		for map_id in 1..10 {
			let p = format!("kcs2/resources/map/{area_id:03}/{map_id:02}.png");
			let exists = cache.exists_on_remote(&p, NoVersion).await?;
			if exists {
				list.add_unversioned(p);
			} else {
				break;
			}
		}
	}

	Ok(())
}
