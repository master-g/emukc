//! K9998 contains abyssal ship quotes files.

use std::sync::LazyLock;

use emukc_model::thirdparty::CacheSource;

use crate::{make_list::CacheList, prelude::CacheListMakeStrategy};

static MISSING_IDS: LazyLock<Vec<u64>> =
	LazyLock::new(|| vec![555213521, 555213531, 555213530, 555213541]);

pub(super) fn make(
	cache_source: &Option<CacheSource>,
	list: &mut CacheList,
	strategy: CacheListMakeStrategy,
) {
	if strategy == CacheListMakeStrategy::Minimal {
		return;
	}

	if let Some(source) = cache_source {
		source.voices.abyssal.iter().for_each(|id| {
			if !MISSING_IDS.contains(id) {
				list.add_unversioned(format!("kcs/sound/kc9998/{id}.mp3"));
			}
		});
	}
}
